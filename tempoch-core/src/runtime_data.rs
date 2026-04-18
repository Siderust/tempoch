// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Opt-in runtime time-data fetching, caching, and use.
//!
//! This module keeps the compile-time generated tables as the default path
//! while exposing an explicit runtime refresh workflow for callers that want
//! current UTC-TAI, modern Delta T, and IERS EOP data.

use crate::constats::{TT_MINUS_TAI, UTC_INTERVAL_EPS};
use crate::delta_t::delta_t_seconds_from_modern_points;
use crate::encoding::{
    j2000_seconds_to_jd, jd_to_j2000_seconds, jd_to_mjd, mjd_to_j2000_seconds, mjd_to_unix_seconds,
    unix_seconds_to_jd, unix_seconds_to_mjd,
};
use crate::eop::EopValues;
use crate::error::ConversionError;
use crate::format::Format;
use crate::format_conversion::CanonicalRoundtrip;
use crate::scale::{Scale, UTC};
use crate::scale_conversion::RuntimeContextScaleConvert;
use crate::time::Time;
use chrono::{DateTime, NaiveDate, Utc};
use qtty::time::{Days, Nanoseconds, Seconds};
use qtty::unit::{Day, Nanosecond, Second as SecondUnit};
use qtty::{Day as DayQuantity, Second};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TEMPOCH_DATA_DIR_ENV: &str = "TEMPOCH_DATA_DIR";
const DEFAULT_SUBDIR: &str = ".tempoch/data";
const BUNDLE_DIR_NAME: &str = "bundle";
const PROVENANCE_FILE: &str = "time_data.provenance.json";
const UTC_TAI_HISTORY_FILE: &str = "UTC-TAI.history";
const DELTA_T_OBSERVED_FILE: &str = "deltat.data";
const DELTA_T_PREDICTIONS_FILE: &str = "deltat.preds";
const EOP_FINALS_FILE: &str = "finals2000A.all";
const FETCH_TIMEOUT_SECS: u64 = 60;
const NANOS_PER_SECOND: Nanoseconds = Nanoseconds::new(1_000_000_000.0);

pub const UTC_TAI_HISTORY_URL: &str = "https://hpiers.obspm.fr/eoppc/bul/bulc/UTC-TAI.history";
pub const DELTA_T_OBSERVED_URL: &str = "https://maia.usno.navy.mil/ser7/deltat.data";
pub const DELTA_T_PREDICTIONS_URL: &str = "https://maia.usno.navy.mil/ser7/deltat.preds";
pub const EOP_FINALS_URL: &str = "https://datacenter.iers.org/data/9/finals2000A.all";
pub const PRE_1961_TAI_MINUS_UTC_APPROX: f64 = 10.0;

#[derive(Debug)]
pub enum TimeDataError {
    Io(std::io::Error),
    Download(String),
    Parse(String),
    Integrity(String),
}

impl fmt::Display for TimeDataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Download(msg) => write!(f, "download error: {msg}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::Integrity(msg) => write!(f, "integrity error: {msg}"),
        }
    }
}

impl std::error::Error for TimeDataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TimeDataError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeDataProvenance {
    fetched_utc: String,
    utc_tai_sha256: String,
    delta_t_observed_sha256: String,
    delta_t_predictions_sha256: String,
    eop_finals_sha256: String,
}

impl TimeDataProvenance {
    pub fn fetched_utc(&self) -> &str {
        &self.fetched_utc
    }

    pub fn utc_tai_sha256(&self) -> &str {
        &self.utc_tai_sha256
    }

    pub fn delta_t_observed_sha256(&self) -> &str {
        &self.delta_t_observed_sha256
    }

    pub fn delta_t_predictions_sha256(&self) -> &str {
        &self.delta_t_predictions_sha256
    }

    pub fn eop_finals_sha256(&self) -> &str {
        &self.eop_finals_sha256
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtcTaiSegment {
    pub start_mjd: i32,
    pub end_mjd: Option<i32>,
    pub base_seconds: f64,
    pub reference_mjd: f64,
    pub slope_seconds_per_day: f64,
}

impl UtcTaiSegment {
    fn start_mjd_days(self) -> DayQuantity {
        DayQuantity::new(self.start_mjd as f64)
    }

    fn end_mjd_days(self) -> Option<DayQuantity> {
        self.end_mjd.map(|mjd| DayQuantity::new(mjd as f64))
    }

    fn reference_mjd_days(self) -> DayQuantity {
        DayQuantity::new(self.reference_mjd)
    }

    fn offset_at(self, mjd_utc: DayQuantity) -> Second {
        let utc_offset = mjd_utc - self.reference_mjd_days();
        Second::new(self.base_seconds)
            + Second::new(self.slope_seconds_per_day) * (utc_offset / DayQuantity::new(1.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EopPoint {
    pub mjd: i32,
    pub pm_observed: bool,
    pub ut1_observed: bool,
    pub nutation_observed: bool,
    pub pm_xp_arcsec: Option<f64>,
    pub pm_yp_arcsec: Option<f64>,
    pub ut1_minus_utc_seconds: f64,
    pub lod_milliseconds: Option<f64>,
    pub dx_milliarcsec: Option<f64>,
    pub dy_milliarcsec: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct RuntimeTimeData {
    utc_tai_segments: Vec<UtcTaiSegment>,
    modern_delta_t_points: Vec<(f64, f64)>,
    modern_delta_t_observed_end_mjd: f64,
    eop_points: Vec<EopPoint>,
    provenance: TimeDataProvenance,
}

impl RuntimeTimeData {
    pub fn utc_tai_segments(&self) -> &[UtcTaiSegment] {
        &self.utc_tai_segments
    }

    pub fn modern_delta_t_points(&self) -> &[(f64, f64)] {
        &self.modern_delta_t_points
    }

    pub fn eop_points(&self) -> &[EopPoint] {
        &self.eop_points
    }

    pub fn provenance(&self) -> &TimeDataProvenance {
        &self.provenance
    }

    pub fn utc_tai_history_start_mjd(&self) -> DayQuantity {
        DayQuantity::new(self.utc_tai_segments[0].start_mjd as f64)
    }

    pub fn delta_t_prediction_horizon_mjd(&self) -> DayQuantity {
        DayQuantity::new(self.modern_delta_t_points.last().unwrap().0)
    }

    pub fn modern_delta_t_observed_end_mjd(&self) -> DayQuantity {
        DayQuantity::new(self.modern_delta_t_observed_end_mjd)
    }

    pub fn eop_start_mjd(&self) -> DayQuantity {
        DayQuantity::new(self.eop_points[0].mjd as f64)
    }

    pub fn eop_observed_end_mjd(&self) -> DayQuantity {
        DayQuantity::new(observed_end_mjd(&self.eop_points) as f64)
    }

    pub fn eop_end_mjd(&self) -> DayQuantity {
        DayQuantity::new(self.eop_points.last().unwrap().mjd as f64)
    }

    pub fn context(&self) -> RuntimeTimeContext {
        RuntimeTimeContext::from_shared(Arc::new(self.clone()))
    }

    fn from_raw_sources(
        utc_tai_history: &str,
        delta_t_observed: &str,
        delta_t_predictions: &str,
        eop_finals: &str,
        provenance: TimeDataProvenance,
    ) -> Result<Self, TimeDataError> {
        let utc_tai_segments =
            parse_utc_tai_segments(utc_tai_history).map_err(TimeDataError::Parse)?;
        let observed = parse_delta_t_observed(delta_t_observed).map_err(TimeDataError::Parse)?;
        let predicted =
            parse_delta_t_predictions(delta_t_predictions).map_err(TimeDataError::Parse)?;
        let (modern_delta_t_points, modern_delta_t_observed_end_mjd) =
            build_modern_delta_t_points(&observed, &predicted).map_err(TimeDataError::Parse)?;
        let eop_points = parse_eop_finals(eop_finals).map_err(TimeDataError::Parse)?;
        Ok(Self {
            utc_tai_segments,
            modern_delta_t_points,
            modern_delta_t_observed_end_mjd,
            eop_points,
            provenance,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeTimeContext {
    data: Arc<RuntimeTimeData>,
}

impl RuntimeTimeContext {
    pub fn new(data: RuntimeTimeData) -> Self {
        Self {
            data: Arc::new(data),
        }
    }

    pub fn from_shared(data: Arc<RuntimeTimeData>) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &RuntimeTimeData {
        self.data.as_ref()
    }

    pub fn eop_at(&self, mjd_utc: DayQuantity) -> Option<EopValues> {
        eop_at_from_points(&self.data.eop_points, mjd_utc)
    }

    pub fn ut1_minus_utc(&self, mjd_utc: DayQuantity) -> Option<Second> {
        self.eop_at(mjd_utc).map(|v| v.ut1_minus_utc)
    }

    pub fn delta_t_seconds(&self, jd_ut: DayQuantity) -> Result<Second, ConversionError> {
        delta_t_seconds_from_modern_points(jd_ut, &self.data.modern_delta_t_points)
    }

    pub fn utc_tai_history_start_mjd(&self) -> DayQuantity {
        self.data.utc_tai_history_start_mjd()
    }

    pub fn delta_t_prediction_horizon_mjd(&self) -> DayQuantity {
        self.data.delta_t_prediction_horizon_mjd()
    }

    pub fn eop_start_mjd(&self) -> DayQuantity {
        self.data.eop_start_mjd()
    }

    pub fn eop_observed_end_mjd(&self) -> DayQuantity {
        self.data.eop_observed_end_mjd()
    }

    pub fn eop_end_mjd(&self) -> DayQuantity {
        self.data.eop_end_mjd()
    }

    pub(crate) fn try_tai_minus_utc_mjd(&self, mjd_utc: DayQuantity) -> Option<Second> {
        try_tai_minus_utc_mjd_with_segments(&self.data.utc_tai_segments, mjd_utc)
    }
}

pub struct TimeDataManager {
    data_dir: PathBuf,
}

impl TimeDataManager {
    pub fn new() -> Result<Self, TimeDataError> {
        let data_dir = resolve_data_dir()?;
        fs::create_dir_all(&data_dir)?;
        Ok(Self { data_dir })
    }

    pub fn with_dir(dir: impl Into<PathBuf>) -> Result<Self, TimeDataError> {
        let data_dir = dir.into();
        fs::create_dir_all(&data_dir)?;
        Ok(Self { data_dir })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn load_cached(&self) -> Result<RuntimeTimeData, TimeDataError> {
        load_cached_bundle(self.bundle_dir())
    }

    pub fn refresh(&self) -> Result<(), TimeDataError> {
        fs::create_dir_all(&self.data_dir)?;
        let staging_dir = self.staging_dir();
        if staging_dir.exists() {
            fs::remove_dir_all(&staging_dir)?;
        }
        fs::create_dir_all(&staging_dir)?;

        let fetch_ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let utc_tai = fetch_text(UTC_TAI_HISTORY_URL)?;
        let delta_obs = fetch_text(DELTA_T_OBSERVED_URL)?;
        let delta_pred = fetch_text(DELTA_T_PREDICTIONS_URL)?;
        let eop = fetch_text(EOP_FINALS_URL)?;

        fs::write(staging_dir.join(UTC_TAI_HISTORY_FILE), &utc_tai.text)?;
        fs::write(staging_dir.join(DELTA_T_OBSERVED_FILE), &delta_obs.text)?;
        fs::write(staging_dir.join(DELTA_T_PREDICTIONS_FILE), &delta_pred.text)?;
        fs::write(staging_dir.join(EOP_FINALS_FILE), &eop.text)?;

        let provenance = TimeDataProvenance {
            fetched_utc: fetch_ts,
            utc_tai_sha256: utc_tai.sha256,
            delta_t_observed_sha256: delta_obs.sha256,
            delta_t_predictions_sha256: delta_pred.sha256,
            eop_finals_sha256: eop.sha256,
        };
        fs::write(
            staging_dir.join(PROVENANCE_FILE),
            render_provenance_json(&provenance),
        )?;

        load_cached_bundle(staging_dir.clone())?;
        swap_bundle_dirs(&staging_dir, self.bundle_dir())?;
        Ok(())
    }

    pub fn refresh_and_load(&self) -> Result<RuntimeTimeData, TimeDataError> {
        self.refresh()?;
        self.load_cached()
    }

    fn bundle_dir(&self) -> PathBuf {
        self.data_dir.join(BUNDLE_DIR_NAME)
    }

    fn staging_dir(&self) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        self.data_dir.join(format!(
            ".{BUNDLE_DIR_NAME}.staging-{}-{nonce}",
            std::process::id()
        ))
    }
}

struct DownloadedText {
    text: String,
    sha256: String,
}

fn resolve_data_dir() -> Result<PathBuf, TimeDataError> {
    if let Ok(dir) = std::env::var(TEMPOCH_DATA_DIR_ENV) {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| {
            TimeDataError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cannot determine home directory. Set TEMPOCH_DATA_DIR explicitly.",
            ))
        })?;

    Ok(PathBuf::from(home).join(DEFAULT_SUBDIR))
}

fn load_cached_bundle(bundle_dir: PathBuf) -> Result<RuntimeTimeData, TimeDataError> {
    if !bundle_dir.exists() {
        return Err(TimeDataError::Integrity(format!(
            "cached bundle not found at {}",
            bundle_dir.display()
        )));
    }

    let utc_tai_history = read_text(bundle_dir.join(UTC_TAI_HISTORY_FILE))?;
    let delta_t_observed = read_text(bundle_dir.join(DELTA_T_OBSERVED_FILE))?;
    let delta_t_predictions = read_text(bundle_dir.join(DELTA_T_PREDICTIONS_FILE))?;
    let eop_finals = read_text(bundle_dir.join(EOP_FINALS_FILE))?;
    let provenance_text = read_text(bundle_dir.join(PROVENANCE_FILE))?;
    let provenance = parse_provenance_json(&provenance_text)?;

    verify_sha256(
        "UTC-TAI history",
        &utc_tai_history,
        provenance.utc_tai_sha256(),
    )?;
    verify_sha256(
        "Delta T observed",
        &delta_t_observed,
        provenance.delta_t_observed_sha256(),
    )?;
    verify_sha256(
        "Delta T predictions",
        &delta_t_predictions,
        provenance.delta_t_predictions_sha256(),
    )?;
    verify_sha256("EOP finals", &eop_finals, provenance.eop_finals_sha256())?;

    RuntimeTimeData::from_raw_sources(
        &utc_tai_history,
        &delta_t_observed,
        &delta_t_predictions,
        &eop_finals,
        provenance,
    )
}

fn swap_bundle_dirs(staging_dir: &Path, live_dir: PathBuf) -> Result<(), TimeDataError> {
    let backup_dir = live_dir.with_extension("backup");
    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir)?;
    }
    if live_dir.exists() {
        fs::rename(&live_dir, &backup_dir)?;
    }
    match fs::rename(staging_dir, &live_dir) {
        Ok(()) => {
            if backup_dir.exists() {
                fs::remove_dir_all(&backup_dir)?;
            }
            Ok(())
        }
        Err(err) => {
            if backup_dir.exists() && !live_dir.exists() {
                let _ = fs::rename(&backup_dir, &live_dir);
            }
            Err(TimeDataError::Io(err))
        }
    }
}

fn read_text(path: PathBuf) -> Result<String, TimeDataError> {
    fs::read_to_string(&path).map_err(|err| {
        TimeDataError::Io(std::io::Error::new(
            err.kind(),
            format!("{}: {err}", path.display()),
        ))
    })
}

fn fetch_text(url: &str) -> Result<DownloadedText, TimeDataError> {
    let response = ureq::get(url)
        .set("User-Agent", "tempoch-runtime-data/1.0")
        .timeout(std::time::Duration::from_secs(FETCH_TIMEOUT_SECS))
        .call()
        .map_err(|err| TimeDataError::Download(format!("fetch {url} failed: {err}")))?;
    let bytes = {
        let mut buf = Vec::new();
        let mut reader = response.into_reader();
        reader
            .read_to_end(&mut buf)
            .map_err(|err| TimeDataError::Download(format!("read {url} body failed: {err}")))?;
        buf
    };
    let text = String::from_utf8(bytes.clone())
        .map_err(|err| TimeDataError::Download(format!("{url} is not UTF-8: {err}")))?;
    Ok(DownloadedText {
        text,
        sha256: sha256_bytes(&bytes),
    })
}

fn render_provenance_json(provenance: &TimeDataProvenance) -> String {
    let value = serde_json::json!({
        "fetched_utc": provenance.fetched_utc(),
        "utc_tai_sha256": provenance.utc_tai_sha256(),
        "delta_t_observed_sha256": provenance.delta_t_observed_sha256(),
        "delta_t_predictions_sha256": provenance.delta_t_predictions_sha256(),
        "eop_finals_sha256": provenance.eop_finals_sha256(),
    });
    let mut rendered =
        serde_json::to_string_pretty(&value).expect("serializing time-data provenance should work");
    rendered.push('\n');
    rendered
}

fn parse_provenance_json(text: &str) -> Result<TimeDataProvenance, TimeDataError> {
    let json: Value =
        serde_json::from_str(text).map_err(|err| TimeDataError::Integrity(err.to_string()))?;
    let string_field = |name: &str| -> Result<String, TimeDataError> {
        json.get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| TimeDataError::Integrity(format!("missing provenance field {name}")))
    };
    Ok(TimeDataProvenance {
        fetched_utc: string_field("fetched_utc")?,
        utc_tai_sha256: string_field("utc_tai_sha256")?,
        delta_t_observed_sha256: string_field("delta_t_observed_sha256")?,
        delta_t_predictions_sha256: string_field("delta_t_predictions_sha256")?,
        eop_finals_sha256: string_field("eop_finals_sha256")?,
    })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn verify_sha256(label: &str, text: &str, expected: &str) -> Result<(), TimeDataError> {
    let actual = sha256_bytes(text.as_bytes());
    if actual != expected {
        return Err(TimeDataError::Integrity(format!(
            "{label} SHA-256 mismatch: expected {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn mjd_epoch() -> NaiveDate {
    NaiveDate::from_ymd_opt(1858, 11, 17).unwrap()
}

fn mjd_from_date(d: NaiveDate) -> i32 {
    (d - mjd_epoch()).num_days() as i32
}

fn normalize_ws(s: &str) -> String {
    s.replace('\t', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_month(token: &str) -> Result<u32, String> {
    let key: String = token
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let month = match key.as_str() {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return Err(format!("unknown month token: {token:?}")),
    };
    Ok(month)
}

fn parse_date_fragment(fragment: &str, default_year: Option<i32>) -> Result<NaiveDate, String> {
    let normalized = normalize_ws(fragment);
    let normalized = normalized.trim_end_matches('.').trim();
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    let (year, month_token, day_token) = match tokens.as_slice() {
        [year, month, day] if year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()) => (
            year.parse::<i32>().map_err(|err| err.to_string())?,
            *month,
            *day,
        ),
        [month, day] => (
            default_year.ok_or_else(|| format!("missing year for fragment: {fragment:?}"))?,
            *month,
            *day,
        ),
        _ => return Err(format!("unable to parse date fragment: {fragment:?}")),
    };
    let month = parse_month(month_token)?;
    let day = day_token
        .parse::<u32>()
        .map_err(|_| format!("bad day in fragment: {fragment:?}"))?;
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| format!("invalid calendar date in fragment: {fragment:?}"))
}

fn compact_number(s: &str) -> Result<f64, String> {
    s.replace(' ', "")
        .parse::<f64>()
        .map_err(|err| format!("bad number {s:?}: {err}"))
}

fn extract_base_seconds(formula: &str) -> Result<f64, String> {
    let bytes = formula.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b's' {
            let mut start = index;
            while start > 0 {
                let c = bytes[start - 1];
                if c.is_ascii_digit() || c == b'.' || c == b' ' {
                    start -= 1;
                } else {
                    break;
                }
            }
            let candidate = &formula[start..index];
            if candidate.chars().any(|c| c.is_ascii_digit()) {
                return compact_number(candidate);
            }
        }
        index += 1;
    }
    Err(format!("unable to parse TAI-UTC base from {formula:?}"))
}

fn extract_slope(formula: &str) -> Result<Option<(f64, f64)>, String> {
    let Some(mjd_idx) = formula.find("MJD") else {
        return Ok(None);
    };
    let rest = &formula[mjd_idx + 3..];
    let rest = rest.trim_start();
    if !rest.starts_with('-') {
        return Ok(None);
    }
    let after_dash = rest[1..].trim_start();
    let ref_end = after_dash
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == ' '))
        .map(|(idx, _)| idx)
        .unwrap_or(after_dash.len());
    let ref_str = after_dash[..ref_end].trim();
    if ref_str.is_empty() {
        return Ok(None);
    }
    let reference_mjd = compact_number(ref_str)?;
    let after_ref = after_dash[ref_end..].trim_start();
    let after_paren = after_ref
        .strip_prefix(')')
        .unwrap_or(after_ref)
        .trim_start();
    let after_x = match after_paren.strip_prefix('x') {
        Some(rest) => rest.trim_start(),
        None => return Ok(None),
    };
    let slope_end = after_x
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.' || *c == ' '))
        .map(|(idx, _)| idx)
        .unwrap_or(after_x.len());
    let slope_str = after_x[..slope_end].trim();
    if slope_str.is_empty() {
        return Ok(None);
    }
    let rest_after_slope = after_x[slope_end..].trim_start();
    if !rest_after_slope.starts_with('s') {
        return Ok(None);
    }
    let slope = compact_number(slope_str)?;
    Ok(Some((reference_mjd, slope)))
}

pub fn parse_utc_tai_segments(text: &str) -> Result<Vec<UtcTaiSegment>, String> {
    let mut segments = Vec::new();
    let mut previous_end: Option<NaiveDate> = None;
    let mut previous_reference_mjd: Option<f64> = None;
    let mut previous_slope: Option<f64> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        if !line.contains('-')
            || line.contains("UTC-TAI.history")
            || line.contains("Limits of validity")
        {
            continue;
        }
        if !line.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }

        let dash_idx = line.find('-').unwrap();
        let (left, right) = line.split_at(dash_idx);
        let right = &right[1..];
        if !left.chars().any(|c| c.is_ascii_alphabetic()) {
            continue;
        }

        let default_start_year = previous_end.map(date_year);
        let start_date = parse_date_fragment(left, default_start_year)?;
        let right_normalized = normalize_ws(right);
        let (end_date, formula) = match parse_end_and_formula(&right_normalized, start_date) {
            Some((end_date, formula)) => (Some(end_date), formula),
            None => (None, right_normalized.clone()),
        };

        let base_seconds = extract_base_seconds(&formula)?;
        let (reference_mjd, slope_seconds_per_day) =
            if let Some((reference_mjd, slope)) = extract_slope(&formula)? {
                (reference_mjd, slope)
            } else if formula.contains("\"\"") {
                match (previous_reference_mjd, previous_slope) {
                    (Some(reference_mjd), Some(slope)) => (reference_mjd, slope),
                    _ => {
                        return Err(format!(
                            "repeated UTC formula without previous state: {formula:?}"
                        ))
                    }
                }
            } else {
                (mjd_from_date(start_date) as f64, 0.0)
            };

        segments.push(UtcTaiSegment {
            start_mjd: mjd_from_date(start_date),
            end_mjd: end_date.map(mjd_from_date),
            base_seconds,
            reference_mjd,
            slope_seconds_per_day,
        });

        previous_end = end_date;
        previous_reference_mjd = Some(reference_mjd);
        previous_slope = Some(slope_seconds_per_day);
    }

    if segments.is_empty() {
        return Err("UTC-TAI history parsing produced no segments".into());
    }
    Ok(segments)
}

fn date_year(date: NaiveDate) -> i32 {
    use chrono::Datelike;
    date.year()
}

fn parse_end_and_formula(
    right_normalized: &str,
    start_date: NaiveDate,
) -> Option<(NaiveDate, String)> {
    let tokens: Vec<&str> = right_normalized.splitn(4, ' ').collect();
    if tokens.len() < 3 {
        return None;
    }
    if tokens.len() == 4
        && tokens[0].len() == 4
        && tokens[0].chars().all(|c| c.is_ascii_digit())
        && parse_month(tokens[1]).is_ok()
        && !tokens[2].is_empty()
        && tokens[2].chars().all(|c| c.is_ascii_digit())
    {
        let year = tokens[0].parse::<i32>().ok()?;
        let month = parse_month(tokens[1]).ok()?;
        let day = tokens[2].parse::<u32>().ok()?;
        let end_date = NaiveDate::from_ymd_opt(year, month, day)?;
        return Some((end_date, tokens[3].to_string()));
    }
    if parse_month(tokens[0]).is_ok()
        && !tokens[1].is_empty()
        && tokens[1].chars().all(|c| c.is_ascii_digit())
    {
        let month = parse_month(tokens[0]).ok()?;
        let day = tokens[1].parse::<u32>().ok()?;
        let end_date = NaiveDate::from_ymd_opt(date_year(start_date), month, day)?;
        let rest = right_normalized
            .splitn(3, ' ')
            .nth(2)
            .unwrap_or("")
            .to_string();
        return Some((end_date, rest));
    }
    None
}

pub fn parse_delta_t_observed(text: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut points = Vec::new();
    for raw_line in text.lines() {
        let parts: Vec<&str> = raw_line.split_whitespace().collect();
        if parts.len() != 4 {
            continue;
        }
        if !parts[0].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let year = parts[0]
            .parse::<i32>()
            .map_err(|err: std::num::ParseIntError| err.to_string())?;
        let month = parts[1]
            .parse::<u32>()
            .map_err(|err: std::num::ParseIntError| err.to_string())?;
        let day = parts[2]
            .parse::<u32>()
            .map_err(|err: std::num::ParseIntError| err.to_string())?;
        let delta_t = parts[3]
            .parse::<f64>()
            .map_err(|err: std::num::ParseFloatError| err.to_string())?;
        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| format!("invalid date in observed Delta T: {raw_line:?}"))?;
        points.push((mjd_from_date(date) as f64, delta_t));
    }
    if points.is_empty() {
        return Err("observed Delta T parsing produced no points".into());
    }
    Ok(points)
}

pub fn parse_delta_t_predictions(text: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut points = Vec::new();
    for raw_line in text.lines() {
        let parts: Vec<&str> = raw_line.split_whitespace().collect();
        if parts.is_empty() || parts[0] == "MJD" || parts.len() < 3 {
            continue;
        }
        let Ok(mjd) = parts[0].parse::<f64>() else {
            continue;
        };
        let Ok(delta_t) = parts[2].parse::<f64>() else {
            continue;
        };
        points.push((mjd, delta_t));
    }
    if points.is_empty() {
        return Err("predicted Delta T parsing produced no points".into());
    }
    Ok(points)
}

pub fn build_modern_delta_t_points(
    observed_points: &[(f64, f64)],
    predicted_points: &[(f64, f64)],
) -> Result<(Vec<(f64, f64)>, f64), String> {
    let (last_obs_mjd, last_obs_dt) = *observed_points.last().ok_or("observed Delta T is empty")?;
    let mut future: Vec<(f64, f64)> = predicted_points
        .iter()
        .copied()
        .filter(|(mjd, _)| *mjd > last_obs_mjd)
        .collect();

    if !future.is_empty() {
        let (m0, d0) = future[0];
        let (m1, d1) = if future.len() >= 2 {
            future[1]
        } else {
            (m0, d0)
        };
        let frac = if m1 != m0 {
            (last_obs_mjd - m0) / (m1 - m0)
        } else {
            0.0
        };
        let pred_at_stitch = d0 + frac * (d1 - d0);
        let continuity_offset = last_obs_dt - pred_at_stitch;
        for point in &mut future {
            point.1 += continuity_offset;
        }
    }

    let mut combined = Vec::with_capacity(observed_points.len() + future.len());
    combined.extend_from_slice(observed_points);
    combined.extend_from_slice(&future);
    if combined.len() < 2 {
        return Err("modern Delta T series must contain at least two points".into());
    }
    Ok((combined, last_obs_mjd))
}

pub fn parse_eop_finals(text: &str) -> Result<Vec<EopPoint>, String> {
    let mut points = Vec::new();

    for line in text.lines() {
        if line.len() < 68 {
            continue;
        }
        let Some(mjd_f) = col(line, 8, 15).and_then(parse_f64) else {
            continue;
        };
        let mjd = mjd_f.round() as i32;
        let Some(ut1_flag) = col(line, 58, 58).and_then(parse_flag) else {
            continue;
        };
        if !matches!(ut1_flag, 'I' | 'P') {
            continue;
        }
        let Some(ut1_minus_utc_seconds) = col(line, 59, 68).and_then(parse_f64) else {
            continue;
        };

        let pm_flag = col(line, 17, 17).and_then(parse_flag);
        let nutation_flag = col(line, 96, 96).and_then(parse_flag);
        points.push(EopPoint {
            mjd,
            pm_observed: matches!(pm_flag, Some('I')),
            ut1_observed: ut1_flag == 'I',
            nutation_observed: matches!(nutation_flag, Some('I')),
            pm_xp_arcsec: col(line, 19, 27).and_then(parse_f64),
            pm_yp_arcsec: col(line, 38, 46).and_then(parse_f64),
            ut1_minus_utc_seconds,
            lod_milliseconds: col(line, 80, 86).and_then(parse_f64),
            dx_milliarcsec: col(line, 98, 106).and_then(parse_f64),
            dy_milliarcsec: col(line, 117, 125).and_then(parse_f64),
        });
    }

    if points.len() < 2 {
        return Err("EOP finals parsing produced fewer than two usable rows".into());
    }
    for window in points.windows(2) {
        if window[1].mjd <= window[0].mjd {
            return Err(format!(
                "EOP finals MJD column is not strictly increasing near {} -> {}",
                window[0].mjd, window[1].mjd
            ));
        }
    }

    Ok(points)
}

pub fn observed_end_mjd(points: &[EopPoint]) -> i32 {
    points
        .iter()
        .rev()
        .find(|point| point.ut1_observed)
        .map(|point| point.mjd)
        .unwrap_or(points[0].mjd)
}

fn col(line: &str, start_1based: usize, end_1based_inclusive: usize) -> Option<&str> {
    let start = start_1based.checked_sub(1)?;
    let end = end_1based_inclusive;
    if line.len() < end {
        return None;
    }
    Some(&line[start..end])
}

fn parse_f64(slice: &str) -> Option<f64> {
    let trimmed = slice.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<f64>().ok()
}

fn parse_flag(slice: &str) -> Option<char> {
    slice.trim().chars().next()
}

fn eop_at_from_points(points: &[EopPoint], mjd_utc: DayQuantity) -> Option<EopValues> {
    let mjd_f = mjd_utc.value();
    let lo_i = mjd_f.floor() as i32;
    let hi_i = lo_i + 1;
    let first = points[0].mjd;
    let last = points[points.len() - 1].mjd;
    if lo_i < first || lo_i > last {
        return None;
    }
    let lo_idx = (lo_i - first) as usize;
    let hi_idx = if hi_i > last {
        lo_idx
    } else {
        (hi_i - first) as usize
    };
    let lo = points[lo_idx];
    let hi = points[hi_idx];
    let frac = if lo_idx == hi_idx {
        0.0
    } else {
        mjd_f - lo_i as f64
    };
    let lerp = |a: f64, b: f64| a + frac * (b - a);
    let lerp_opt = |a: Option<f64>, b: Option<f64>| match (a, b) {
        (Some(a), Some(b)) => Some(lerp(a, b)),
        _ => None,
    };
    let lod_milliseconds = match (lo.lod_milliseconds, hi.lod_milliseconds) {
        (Some(a), Some(b)) => Some(lerp(a, b)),
        _ => None,
    };

    Some(EopValues {
        mjd_utc,
        pm_xp_arcsec: lerp_opt(lo.pm_xp_arcsec, hi.pm_xp_arcsec),
        pm_yp_arcsec: lerp_opt(lo.pm_yp_arcsec, hi.pm_yp_arcsec),
        ut1_minus_utc: Second::new(lerp(lo.ut1_minus_utc_seconds, hi.ut1_minus_utc_seconds)),
        lod_milliseconds,
        dx_milliarcsec: lerp_opt(lo.dx_milliarcsec, hi.dx_milliarcsec),
        dy_milliarcsec: lerp_opt(lo.dy_milliarcsec, hi.dy_milliarcsec),
        ut1_observed: lo.ut1_observed && hi.ut1_observed,
    })
}

fn utc_offset_seconds_in_segment(mjd_utc: Days, segment: UtcTaiSegment) -> Seconds {
    segment.offset_at(mjd_utc)
}

fn utc_mjd_to_tt_mjd_in_segment(mjd_utc: Days, segment: UtcTaiSegment) -> Days {
    mjd_utc + (utc_offset_seconds_in_segment(mjd_utc, segment) + TT_MINUS_TAI).to::<Day>()
}

fn tt_mjd_to_utc_mjd_in_segment(mjd_tt: Days, segment: UtcTaiSegment) -> Days {
    let scale = Days::new(1.0) + Seconds::new(segment.slope_seconds_per_day).to::<Day>();
    let ref_days = segment.reference_mjd_days() / Days::new(1.0);
    let offset_days = (Seconds::new(segment.base_seconds)
        - Seconds::new(segment.slope_seconds_per_day) * ref_days
        + TT_MINUS_TAI)
        .to::<Day>();
    Days::new((mjd_tt - offset_days) / scale)
}

fn try_tai_minus_utc_mjd_with_segments(
    segments: &[UtcTaiSegment],
    mjd_utc: Days,
) -> Option<Seconds> {
    if mjd_utc < DayQuantity::new(segments[0].start_mjd as f64) {
        return None;
    }
    let idx = segments.partition_point(|segment| segment.start_mjd_days() <= mjd_utc);
    let segment = segments[idx - 1];
    Some(utc_offset_seconds_in_segment(mjd_utc, segment))
}

#[derive(Clone, Copy)]
enum UtcTaiRegion {
    Segment(UtcTaiSegment),
    Leap {
        end_mjd: Days,
        end_tt: Days,
        next_start_tt: Days,
    },
}

fn segment_start_tt(segment: UtcTaiSegment) -> Days {
    utc_mjd_to_tt_mjd_in_segment(segment.start_mjd_days(), segment)
}

fn locate_utc_region_from_tt_mjd(
    segments: &[UtcTaiSegment],
    mjd_tt: Days,
) -> Result<UtcTaiRegion, ConversionError> {
    let first = segments[0];
    if mjd_tt < segment_start_tt(first) - UTC_INTERVAL_EPS {
        return Err(ConversionError::UtcHistoryUnsupported);
    }

    let idx =
        segments.partition_point(|segment| segment_start_tt(*segment) <= mjd_tt + UTC_INTERVAL_EPS);
    let segment = segments[idx.saturating_sub(1)];
    if let Some(end_mjd) = segment.end_mjd_days() {
        let end_tt = utc_mjd_to_tt_mjd_in_segment(end_mjd, segment);
        if mjd_tt >= end_tt - UTC_INTERVAL_EPS {
            if let Some(next) = segments.get(idx).copied() {
                let next_start_tt = segment_start_tt(next);
                if mjd_tt < next_start_tt - UTC_INTERVAL_EPS {
                    return Ok(UtcTaiRegion::Leap {
                        end_mjd,
                        end_tt,
                        next_start_tt,
                    });
                }
            }
        }
    }

    Ok(UtcTaiRegion::Segment(segment))
}

fn datetime_from_seconds_since_epoch(seconds_since_epoch: Seconds) -> Option<DateTime<Utc>> {
    if !seconds_since_epoch.is_finite() {
        return None;
    }

    let mut secs = seconds_since_epoch.floor();
    let mut nanos: Nanoseconds = (seconds_since_epoch - secs).to::<Nanosecond>().round();
    if nanos >= NANOS_PER_SECOND {
        secs += Seconds::one();
        nanos -= NANOS_PER_SECOND;
    }

    DateTime::<Utc>::from_timestamp(
        (secs / Seconds::one()) as i64,
        (nanos / Nanoseconds::one()) as u32,
    )
}

fn datetime_from_utc_mjd(mjd_utc: Days) -> Option<DateTime<Utc>> {
    datetime_from_seconds_since_epoch(mjd_to_unix_seconds(mjd_utc))
}

fn utc_from_tai_seconds(
    segments: &[UtcTaiSegment],
    tai_secs: Seconds,
) -> Result<DateTime<Utc>, ConversionError> {
    if !tai_secs.is_finite() {
        return Err(ConversionError::NonFinite);
    }
    let jd_tt = j2000_seconds_to_jd(tai_secs + TT_MINUS_TAI);
    let mjd_tt = jd_to_mjd(jd_tt);
    match locate_utc_region_from_tt_mjd(segments, mjd_tt)? {
        UtcTaiRegion::Segment(segment) => {
            let mjd_utc = tt_mjd_to_utc_mjd_in_segment(mjd_tt, segment);
            datetime_from_utc_mjd(mjd_utc).ok_or(ConversionError::OutOfRange)
        }
        UtcTaiRegion::Leap {
            end_mjd,
            end_tt,
            next_start_tt,
        } => {
            let boundary = datetime_from_utc_mjd(end_mjd).ok_or(ConversionError::OutOfRange)?;
            let base_secs = boundary.timestamp() - 1;
            let leap_nanos: Nanoseconds =
                NANOS_PER_SECOND + (mjd_tt - end_tt).to::<SecondUnit>().to::<Nanosecond>();
            let window_nanos: Nanoseconds = (next_start_tt - end_tt)
                .to::<SecondUnit>()
                .to::<Nanosecond>()
                .round()
                .max(Nanoseconds::one());
            let max_nanos = NANOS_PER_SECOND + window_nanos - Nanoseconds::one();
            let nanos = leap_nanos.round().clamp(NANOS_PER_SECOND, max_nanos);
            DateTime::<Utc>::from_timestamp(base_secs, (nanos / Nanoseconds::one()) as u32)
                .ok_or(ConversionError::OutOfRange)
        }
    }
}

fn tai_seconds_from_utc(
    segments: &[UtcTaiSegment],
    dt: DateTime<Utc>,
) -> Result<Second, ConversionError> {
    let base_jd_utc = unix_seconds_to_jd(Seconds::new(dt.timestamp() as f64));
    let tai_minus_utc = try_tai_minus_utc_mjd_with_segments(segments, jd_to_mjd(base_jd_utc))
        .ok_or(ConversionError::UtcHistoryUnsupported)?;
    let subsec_nanos = dt.timestamp_subsec_nanos();
    if subsec_nanos >= 1_000_000_000 {
        let next = try_tai_minus_utc_mjd_with_segments(
            segments,
            jd_to_mjd(base_jd_utc) + Seconds::new(1.0).to::<Day>(),
        )
        .ok_or(ConversionError::InvalidLeapSecond)?;
        if next - tai_minus_utc < Seconds::new(0.5) {
            return Err(ConversionError::InvalidLeapSecond);
        }
    }

    let frac = Nanoseconds::new(subsec_nanos as f64).to::<SecondUnit>();
    Ok(jd_to_j2000_seconds(base_jd_utc) + tai_minus_utc + frac)
}

fn tai_seconds_is_in_leap_window(segments: &[UtcTaiSegment], tai_secs: Second) -> bool {
    let jd_tt = j2000_seconds_to_jd(tai_secs + TT_MINUS_TAI);
    let mjd_tt = jd_to_mjd(jd_tt);
    matches!(
        locate_utc_region_from_tt_mjd(segments, mjd_tt),
        Ok(UtcTaiRegion::Leap { .. })
    )
}

#[allow(private_bounds)]
impl<S: Scale, F: Format + CanonicalRoundtrip> Time<S, F> {
    pub fn to_scale_with_runtime<S2: Scale>(
        self,
        ctx: &RuntimeTimeContext,
    ) -> Result<Time<S2, F>, ConversionError>
    where
        S: RuntimeContextScaleConvert<S2>,
    {
        let src = F::to_j2000s(self.value());
        let dst = <S as RuntimeContextScaleConvert<S2>>::convert_with_runtime(src, ctx)?;
        Ok(Time::new(F::from_j2000s(dst)))
    }
}

#[allow(private_bounds)]
impl<F: Format + CanonicalRoundtrip> Time<UTC, F> {
    pub fn try_from_chrono_with_runtime(
        dt: DateTime<Utc>,
        ctx: &RuntimeTimeContext,
    ) -> Result<Self, ConversionError> {
        let tai_secs = tai_seconds_from_utc(&ctx.data.utc_tai_segments, dt)?;
        Ok(Self::new(F::from_j2000s(tai_secs)))
    }

    #[track_caller]
    pub fn from_chrono_with_runtime(dt: DateTime<Utc>, ctx: &RuntimeTimeContext) -> Self {
        Self::try_from_chrono_with_runtime(dt, ctx)
            .expect("UTC conversion failed; use try_from_chrono_with_runtime")
    }

    pub fn try_to_chrono_with_runtime(
        self,
        ctx: &RuntimeTimeContext,
    ) -> Result<DateTime<Utc>, ConversionError> {
        let tai_secs = F::to_j2000s(self.value());
        utc_from_tai_seconds(&ctx.data.utc_tai_segments, tai_secs)
    }

    pub fn to_chrono_with_runtime(self, ctx: &RuntimeTimeContext) -> Option<DateTime<Utc>> {
        self.try_to_chrono_with_runtime(ctx).ok()
    }

    pub fn from_unix_seconds_with_runtime(
        seconds: Second,
        ctx: &RuntimeTimeContext,
    ) -> Result<Self, ConversionError> {
        if !seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        let mjd_utc = unix_seconds_to_mjd(seconds);
        let tai_minus_utc = ctx
            .try_tai_minus_utc_mjd(mjd_utc)
            .ok_or(ConversionError::UtcHistoryUnsupported)?;
        let tai_secs = mjd_to_j2000_seconds(mjd_utc) + tai_minus_utc;
        Ok(Self::new(F::from_j2000s(tai_secs)))
    }

    pub fn unix_seconds_with_runtime(
        self,
        ctx: &RuntimeTimeContext,
    ) -> Result<Second, ConversionError> {
        let tai_secs = F::to_j2000s(self.value());
        let dt = utc_from_tai_seconds(&ctx.data.utc_tai_segments, tai_secs)?;
        let nanos = dt.timestamp_subsec_nanos().min(999_999_999);
        Ok(Seconds::new(dt.timestamp() as f64) + Nanoseconds::new(nanos as f64).to::<SecondUnit>())
    }

    pub fn is_leap_second_with_runtime(self, ctx: &RuntimeTimeContext) -> bool {
        tai_seconds_is_in_leap_window(&ctx.data.utc_tai_segments, F::to_j2000s(self.value()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::eop_data::EOP_POINTS;
    use crate::generated::time_data::{MODERN_DELTA_T_POINTS, UTC_TAI_SEGMENTS};
    use crate::{JD, Time, TT, UT1};
    use std::sync::Mutex;

    fn compiled_runtime_data() -> RuntimeTimeData {
        let utc_tai_segments = UTC_TAI_SEGMENTS
            .iter()
            .map(|segment| UtcTaiSegment {
                start_mjd: segment.start_mjd,
                end_mjd: segment.end_mjd,
                base_seconds: segment.base_seconds,
                reference_mjd: segment.reference_mjd,
                slope_seconds_per_day: segment.slope_seconds_per_day,
            })
            .collect();
        let modern_delta_t_points = MODERN_DELTA_T_POINTS.to_vec();
        let eop_points = EOP_POINTS
            .iter()
            .map(|point| EopPoint {
                mjd: point.mjd,
                pm_observed: point.pm_observed,
                ut1_observed: point.ut1_observed,
                nutation_observed: point.nutation_observed,
                pm_xp_arcsec: point.pm_xp_arcsec,
                pm_yp_arcsec: point.pm_yp_arcsec,
                ut1_minus_utc_seconds: point.ut1_minus_utc_seconds,
                lod_milliseconds: point.lod_milliseconds,
                dx_milliarcsec: point.dx_milliarcsec,
                dy_milliarcsec: point.dy_milliarcsec,
            })
            .collect();
        RuntimeTimeData {
            utc_tai_segments,
            modern_delta_t_points,
            modern_delta_t_observed_end_mjd: crate::MODERN_DELTA_T_OBSERVED_END_MJD.value(),
            eop_points,
            provenance: TimeDataProvenance {
                fetched_utc: "2026-04-18T14:43:18".to_string(),
                utc_tai_sha256: "compiled".to_string(),
                delta_t_observed_sha256: "compiled".to_string(),
                delta_t_predictions_sha256: "compiled".to_string(),
                eop_finals_sha256: "compiled".to_string(),
            },
        }
    }

    #[test]
    fn parse_utc_tai_segments_parses_flat_line() {
        let text = " 1972  Jan.  1 - 1972  Jul.  1    10s\n";
        let segments = parse_utc_tai_segments(text).unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].start_mjd, 41317);
        assert_eq!(segments[0].end_mjd, Some(41499));
        assert_eq!(segments[0].slope_seconds_per_day, 0.0);
    }

    #[test]
    fn parse_eop_finals_requires_monotone_mjd() {
        let mut row = vec![b' '; 200];
        row[7..15].copy_from_slice(b"60000.00");
        row[57] = b'I';
        row[58..68].copy_from_slice(b" 0.1000000");
        let text = format!(
            "{}\n{}\n",
            String::from_utf8(row.clone()).unwrap(),
            String::from_utf8(row).unwrap()
        );
        let err = parse_eop_finals(&text).unwrap_err();
        assert!(err.contains("not strictly increasing"));
    }

    #[test]
    fn runtime_context_matches_builtin_eop() {
        let data = compiled_runtime_data();
        let ctx = RuntimeTimeContext::new(data);
        let mjd = DayQuantity::new(crate::EOP_START_MJD.value() + 123.25);
        let runtime = ctx.eop_at(mjd).unwrap();
        let builtin = crate::eop::builtin_eop_at(mjd).unwrap();
        assert_eq!(runtime.pm_xp_arcsec, builtin.pm_xp_arcsec);
        assert_eq!(runtime.pm_yp_arcsec, builtin.pm_yp_arcsec);
        assert!((runtime.ut1_minus_utc.value() - builtin.ut1_minus_utc.value()).abs() < 1e-12);
    }

    #[test]
    fn runtime_context_matches_builtin_ut1_conversion() {
        let data = compiled_runtime_data();
        let ctx = RuntimeTimeContext::new(data);
        let tt = Time::<TT, JD>::from_julian_days(DayQuantity::new(2_460_000.25)).unwrap();
        let ut1_runtime: Time<UT1, JD> = tt.to_scale_with_runtime(&ctx).unwrap();
        let ut1_builtin: Time<UT1, JD> = tt
            .to_scale_with(&crate::TimeContext::with_builtin_eop())
            .unwrap();
        assert!(
            (ut1_runtime.julian_days().value() - ut1_builtin.julian_days().value()).abs() < 1e-12
        );
    }

    #[test]
    fn runtime_unix_helpers_match_builtin_path() {
        let data = compiled_runtime_data();
        let ctx = RuntimeTimeContext::new(data);
        let unix = Second::new(1_700_000_000.25);
        let runtime = Time::<UTC>::from_unix_seconds_with_runtime(unix, &ctx).unwrap();
        let builtin = Time::<UTC>::from_unix_seconds(unix).unwrap();
        assert!((runtime.value().value() - builtin.value().value()).abs() < 1e-12);
        let roundtrip = runtime.unix_seconds_with_runtime(&ctx).unwrap();
        assert!((roundtrip - unix).abs() < Second::new(1e-3));
    }

    #[test]
    fn runtime_delta_t_horizon_is_enforced() {
        let data = compiled_runtime_data();
        let ctx = RuntimeTimeContext::new(data);
        let beyond = crate::DELTA_T_PREDICTION_HORIZON_MJD + DayQuantity::new(1.0);
        let jd_ut = beyond + crate::constats::JD_MINUS_MJD;
        assert_eq!(
            ctx.delta_t_seconds(jd_ut).unwrap_err(),
            ConversionError::Ut1HorizonExceeded
        );
    }

    #[test]
    fn manager_refresh_and_load_roundtrip_cached_bundle() {
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard = LOCK.lock().unwrap();

        let dir = std::env::temp_dir().join(format!(
            "tempoch_runtime_data_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join(BUNDLE_DIR_NAME)).unwrap();

        let utc_tai_history = " 1972  Jan.  1 - 1972  Jul.  1    10s\n";
        let delta_t_observed = "1973  2  1   43.4724\n1973  3  1   43.5648\n";
        let delta_t_predictions = "61000.00 2025.50 69.80\n61100.00 2025.77 70.10\n";
        let mut eop_line_a = vec![b' '; 200];
        eop_line_a[7..15].copy_from_slice(b"60000.00");
        eop_line_a[16] = b'I';
        eop_line_a[18..27].copy_from_slice(b" 0.100000");
        eop_line_a[37..46].copy_from_slice(b" 0.200000");
        eop_line_a[57] = b'I';
        eop_line_a[58..68].copy_from_slice(b" 0.1234567");
        let mut eop_line_b = vec![b' '; 200];
        eop_line_b[7..15].copy_from_slice(b"60001.00");
        eop_line_b[16] = b'P';
        eop_line_b[18..27].copy_from_slice(b" 0.100000");
        eop_line_b[37..46].copy_from_slice(b" 0.200000");
        eop_line_b[57] = b'P';
        eop_line_b[58..68].copy_from_slice(b" 0.2234567");
        let eop_finals = format!(
            "{}\n{}\n",
            String::from_utf8(eop_line_a).unwrap(),
            String::from_utf8(eop_line_b).unwrap()
        );

        fs::write(
            dir.join(BUNDLE_DIR_NAME).join(UTC_TAI_HISTORY_FILE),
            utc_tai_history,
        )
        .unwrap();
        fs::write(
            dir.join(BUNDLE_DIR_NAME).join(DELTA_T_OBSERVED_FILE),
            delta_t_observed,
        )
        .unwrap();
        fs::write(
            dir.join(BUNDLE_DIR_NAME).join(DELTA_T_PREDICTIONS_FILE),
            delta_t_predictions,
        )
        .unwrap();
        fs::write(dir.join(BUNDLE_DIR_NAME).join(EOP_FINALS_FILE), &eop_finals).unwrap();
        let provenance = TimeDataProvenance {
            fetched_utc: "2026-04-18T00:00:00".to_string(),
            utc_tai_sha256: sha256_bytes(utc_tai_history.as_bytes()),
            delta_t_observed_sha256: sha256_bytes(delta_t_observed.as_bytes()),
            delta_t_predictions_sha256: sha256_bytes(delta_t_predictions.as_bytes()),
            eop_finals_sha256: sha256_bytes(eop_finals.as_bytes()),
        };
        fs::write(
            dir.join(BUNDLE_DIR_NAME).join(PROVENANCE_FILE),
            render_provenance_json(&provenance),
        )
        .unwrap();

        let manager = TimeDataManager::with_dir(&dir).unwrap();
        let data = manager.load_cached().unwrap();
        assert_eq!(data.utc_tai_segments().len(), 1);
        assert_eq!(data.eop_points().len(), 2);
        fs::remove_dir_all(&dir).ok();
    }
}
