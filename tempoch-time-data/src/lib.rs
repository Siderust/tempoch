// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use chrono::{DateTime, NaiveDate, Utc};
use std::fmt;

pub const TEMPOCH_DATA_DIR_ENV: &str = "TEMPOCH_DATA_DIR";
pub const UTC_TAI_HISTORY_URL: &str = "https://hpiers.obspm.fr/eoppc/bul/bulc/UTC-TAI.history";
pub const DELTA_T_OBSERVED_URL: &str = "https://maia.usno.navy.mil/ser7/deltat.data";
pub const DELTA_T_PREDICTIONS_URL: &str = "https://maia.usno.navy.mil/ser7/deltat.preds";
pub const EOP_FINALS_URL: &str = "https://datacenter.iers.org/data/9/finals2000A.all";
pub const PRE_1961_TAI_MINUS_UTC_APPROX: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtcTaiSegment {
    pub start_mjd: i32,
    pub end_mjd: Option<i32>,
    pub base_seconds: f64,
    pub reference_mjd: f64,
    pub slope_seconds_per_day: f64,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeDataProvenance {
    fetched_utc: String,
    utc_tai_sha256: String,
    delta_t_observed_sha256: String,
    delta_t_predictions_sha256: String,
    eop_finals_sha256: String,
}

impl TimeDataProvenance {
    pub fn new(
        fetched_utc: impl Into<String>,
        utc_tai_sha256: impl Into<String>,
        delta_t_observed_sha256: impl Into<String>,
        delta_t_predictions_sha256: impl Into<String>,
        eop_finals_sha256: impl Into<String>,
    ) -> Self {
        Self {
            fetched_utc: fetched_utc.into(),
            utc_tai_sha256: utc_tai_sha256.into(),
            delta_t_observed_sha256: delta_t_observed_sha256.into(),
            delta_t_predictions_sha256: delta_t_predictions_sha256.into(),
            eop_finals_sha256: eop_finals_sha256.into(),
        }
    }

    pub fn fetched_utc(&self) -> &str {
        &self.fetched_utc
    }

    pub fn fetched_at(&self) -> Option<DateTime<Utc>> {
        chrono::NaiveDateTime::parse_from_str(&self.fetched_utc, "%Y-%m-%dT%H:%M:%S")
            .ok()
            .map(|dt| dt.and_utc())
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

#[derive(Debug, Clone)]
pub struct TimeDataBundle {
    utc_tai_segments: Vec<UtcTaiSegment>,
    modern_delta_t_points: Vec<(f64, f64)>,
    modern_delta_t_observed_end_mjd: f64,
    eop_points: Vec<EopPoint>,
    provenance: TimeDataProvenance,
}

impl TimeDataBundle {
    pub fn new(
        utc_tai_segments: Vec<UtcTaiSegment>,
        modern_delta_t_points: Vec<(f64, f64)>,
        modern_delta_t_observed_end_mjd: f64,
        eop_points: Vec<EopPoint>,
        provenance: TimeDataProvenance,
    ) -> Self {
        Self {
            utc_tai_segments,
            modern_delta_t_points,
            modern_delta_t_observed_end_mjd,
            eop_points,
            provenance,
        }
    }

    pub fn utc_tai_segments(&self) -> &[UtcTaiSegment] {
        &self.utc_tai_segments
    }

    pub fn modern_delta_t_points(&self) -> &[(f64, f64)] {
        &self.modern_delta_t_points
    }

    pub fn modern_delta_t_observed_end_mjd(&self) -> f64 {
        self.modern_delta_t_observed_end_mjd
    }

    pub fn eop_points(&self) -> &[EopPoint] {
        &self.eop_points
    }

    pub fn provenance(&self) -> &TimeDataProvenance {
        &self.provenance
    }

    pub fn eop_observed_end_mjd(&self) -> i32 {
        observed_end_mjd(&self.eop_points)
    }

    pub fn eop_end_mjd(&self) -> i32 {
        self.eop_points
            .last()
            .map(|point| point.mjd)
            .unwrap_or_default()
    }

    #[cfg(feature = "fetch")]
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
        Ok(Self::new(
            utc_tai_segments,
            modern_delta_t_points,
            modern_delta_t_observed_end_mjd,
            eop_points,
            provenance,
        ))
    }
}

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

#[cfg(feature = "fetch")]
mod fetch_support {
    use super::*;
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    const DEFAULT_SUBDIR: &str = ".tempoch/data";
    const BUNDLE_DIR_NAME: &str = "bundle";
    const PROVENANCE_FILE: &str = "time_data.provenance.json";
    const UTC_TAI_HISTORY_FILE: &str = "UTC-TAI.history";
    const DELTA_T_OBSERVED_FILE: &str = "deltat.data";
    const DELTA_T_PREDICTIONS_FILE: &str = "deltat.preds";
    const EOP_FINALS_FILE: &str = "finals2000A.all";
    const FETCH_TIMEOUT_SECS: u64 = 60;

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

        pub fn load_cached(&self) -> Result<TimeDataBundle, TimeDataError> {
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

            let provenance = TimeDataProvenance::new(
                fetch_ts,
                utc_tai.sha256,
                delta_obs.sha256,
                delta_pred.sha256,
                eop.sha256,
            );
            fs::write(
                staging_dir.join(PROVENANCE_FILE),
                render_provenance_json(&provenance),
            )?;

            load_cached_bundle(staging_dir.clone())?;
            swap_bundle_dirs(&staging_dir, self.bundle_dir())?;
            Ok(())
        }

        pub fn refresh_and_load(&self) -> Result<TimeDataBundle, TimeDataError> {
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

    fn load_cached_bundle(bundle_dir: PathBuf) -> Result<TimeDataBundle, TimeDataError> {
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

        TimeDataBundle::from_raw_sources(
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
        let mut rendered = serde_json::to_string_pretty(&value)
            .expect("serializing time-data provenance should work");
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
        Ok(TimeDataProvenance::new(
            string_field("fetched_utc")?,
            string_field("utc_tai_sha256")?,
            string_field("delta_t_observed_sha256")?,
            string_field("delta_t_predictions_sha256")?,
            string_field("eop_finals_sha256")?,
        ))
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
}

#[cfg(feature = "fetch")]
pub use fetch_support::TimeDataManager;

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

    validate_utc_tai_segments(&segments)?;
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

fn validate_utc_tai_segments(segments: &[UtcTaiSegment]) -> Result<(), String> {
    if segments.is_empty() {
        return Err("UTC-TAI history parsing produced no segments".into());
    }
    for (idx, segment) in segments.iter().enumerate() {
        if let Some(end_mjd) = segment.end_mjd {
            if end_mjd <= segment.start_mjd {
                return Err(format!(
                    "UTC-TAI segment ending at MJD {end_mjd} does not extend past start {}",
                    segment.start_mjd
                ));
            }
        }
        let Some(next) = segments.get(idx + 1) else {
            continue;
        };
        if next.start_mjd == segment.start_mjd {
            return Err(format!(
                "UTC-TAI segment list contains duplicate start MJD {}",
                segment.start_mjd
            ));
        }
        if next.start_mjd < segment.start_mjd {
            return Err(format!(
                "UTC-TAI segment list is not strictly increasing near {} -> {}",
                segment.start_mjd, next.start_mjd
            ));
        }
        match segment.end_mjd {
            Some(end_mjd) if end_mjd == next.start_mjd => {}
            Some(end_mjd) => {
                return Err(format!(
                    "UTC-TAI segment boundary mismatch near {} -> {}",
                    end_mjd, next.start_mjd
                ))
            }
            None => {
                return Err(format!(
                    "UTC-TAI segment starting at MJD {} is open-ended before the next segment {}",
                    segment.start_mjd, next.start_mjd
                ))
            }
        }
    }
    Ok(())
}

fn validate_strictly_increasing_mjds(label: &str, points: &[(f64, f64)]) -> Result<(), String> {
    for window in points.windows(2) {
        let current = window[0].0;
        let next = window[1].0;
        if next == current {
            return Err(format!(
                "{label} MJD column contains duplicate entry at {current:.3}"
            ));
        }
        if next < current {
            return Err(format!(
                "{label} MJD column is not strictly increasing near {current:.3} -> {next:.3}"
            ));
        }
    }
    Ok(())
}

fn validate_eop_points(points: &[EopPoint]) -> Result<(), String> {
    if points.len() < 2 {
        return Err("EOP finals parsing produced fewer than two usable rows".into());
    }
    for window in points.windows(2) {
        let current = window[0].mjd;
        let next = window[1].mjd;
        if next == current {
            return Err(format!(
                "EOP finals MJD column contains duplicate entry at {current}"
            ));
        }
        if next < current {
            return Err(format!(
                "EOP finals MJD column is not strictly increasing near {current} -> {next}"
            ));
        }
        if next != current + 1 {
            return Err(format!(
                "EOP finals MJD column has a daily gap near {current} -> {next}"
            ));
        }
    }
    Ok(())
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
    validate_strictly_increasing_mjds("observed Delta T", &points)?;
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
    validate_strictly_increasing_mjds("predicted Delta T", &points)?;
    Ok(points)
}

pub fn build_modern_delta_t_points(
    observed_points: &[(f64, f64)],
    predicted_points: &[(f64, f64)],
) -> Result<(Vec<(f64, f64)>, f64), String> {
    let (last_obs_mjd, last_obs_dt) = *observed_points.last().ok_or("observed Delta T is empty")?;
    validate_strictly_increasing_mjds("observed Delta T", observed_points)?;
    validate_strictly_increasing_mjds("predicted Delta T", predicted_points)?;
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
    validate_strictly_increasing_mjds("modern Delta T", &combined)?;
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

    validate_eop_points(&points)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_utc_tai_history() -> &'static str {
        "1961 Jan. 1 - Aug. 1 1.4228180s + (MJD - 37300) x 0.001296s\n\
         Aug. 1 - 1962 Jan. 1 1.3728180s + \"\"\n\
         1962 Jan. 1 - 10s\n"
    }

    fn set_field(line: &mut [u8], start_1based: usize, end_1based_inclusive: usize, value: &str) {
        let start = start_1based - 1;
        let width = end_1based_inclusive - start_1based + 1;
        let bytes = value.as_bytes();
        assert!(
            bytes.len() <= width,
            "{value:?} does not fit in width {width}"
        );
        let offset = width - bytes.len();
        line[start + offset..start + offset + bytes.len()].copy_from_slice(bytes);
    }

    #[allow(clippy::too_many_arguments)]
    fn sample_eop_line(
        mjd: i32,
        ut1_flag: char,
        ut1_minus_utc_seconds: f64,
        pm_xp_arcsec: Option<f64>,
        pm_yp_arcsec: Option<f64>,
        lod_milliseconds: Option<f64>,
        dx_milliarcsec: Option<f64>,
        dy_milliarcsec: Option<f64>,
    ) -> String {
        let mut line = vec![b' '; 125];
        set_field(&mut line, 8, 15, &format!("{:8.2}", mjd as f64));
        line[16] = b'I';
        if let Some(value) = pm_xp_arcsec {
            set_field(&mut line, 19, 27, &format!("{value:>9.6}"));
        }
        if let Some(value) = pm_yp_arcsec {
            set_field(&mut line, 38, 46, &format!("{value:>9.6}"));
        }
        line[57] = ut1_flag as u8;
        set_field(&mut line, 59, 68, &format!("{ut1_minus_utc_seconds:>10.7}"));
        if let Some(value) = lod_milliseconds {
            set_field(&mut line, 80, 86, &format!("{value:>7.4}"));
        }
        line[95] = b'I';
        if let Some(value) = dx_milliarcsec {
            set_field(&mut line, 98, 106, &format!("{value:>9.3}"));
        }
        if let Some(value) = dy_milliarcsec {
            set_field(&mut line, 117, 125, &format!("{value:>9.3}"));
        }
        String::from_utf8(line).expect("sample EOP line must stay ASCII")
    }

    #[test]
    fn parse_utc_tai_segments_reads_piecewise_rules() {
        let segments = parse_utc_tai_segments(sample_utc_tai_history()).unwrap();
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].start_mjd, 37_300);
        assert_eq!(segments[0].end_mjd, Some(37_512));
        assert_eq!(segments[0].reference_mjd, 37_300.0);
        assert_eq!(segments[0].slope_seconds_per_day, 0.001_296);
        assert_eq!(segments[1].reference_mjd, segments[0].reference_mjd);
        assert_eq!(
            segments[1].slope_seconds_per_day,
            segments[0].slope_seconds_per_day
        );
        assert_eq!(segments[2].end_mjd, None);
        assert_eq!(segments[2].base_seconds, 10.0);
    }

    #[test]
    fn parse_delta_t_observed_reads_representative_rows() {
        let points = parse_delta_t_observed(
            "2024 01 01 69.1000\n\
             2024 02 01 69.2000\n",
        )
        .unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(
            points[0].0,
            mjd_from_date(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()) as f64
        );
        assert_eq!(points[1].1, 69.2);
    }

    #[test]
    fn parse_delta_t_predictions_reads_representative_rows() {
        let points = parse_delta_t_predictions(
            "MJD YEAR DELTAT\n\
             60310 2024.1 69.4000\n\
             60341 2024.2 69.5000\n",
        )
        .unwrap();
        assert_eq!(points, vec![(60_310.0, 69.4), (60_341.0, 69.5)]);
    }

    #[test]
    fn build_modern_delta_t_points_applies_continuity_offset() {
        let observed = [(60_000.0, 69.8), (60_030.0, 71.0)];
        let predicted = [(60_040.0, 70.0), (60_050.0, 72.0)];
        let (combined, observed_end_mjd) =
            build_modern_delta_t_points(&observed, &predicted).unwrap();

        assert_eq!(observed_end_mjd, 60_030.0);
        assert_eq!(combined.len(), 4);
        let (m0, d0) = combined[2];
        let (m1, d1) = combined[3];
        let frac = (observed_end_mjd - m0) / (m1 - m0);
        let stitched_value = d0 + frac * (d1 - d0);
        assert!((stitched_value - 71.0).abs() < 1e-12);
    }

    #[test]
    fn build_modern_delta_t_points_rejects_duplicate_input_mjds() {
        let observed = [(60_000.0, 69.8), (60_000.0, 69.9)];
        let predicted = [(60_031.0, 70.0), (60_062.0, 70.2)];
        let err = build_modern_delta_t_points(&observed, &predicted).unwrap_err();
        assert!(err.contains("observed Delta T"));
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn parse_delta_t_predictions_rejects_non_increasing_mjds() {
        let err = parse_delta_t_predictions(
            "MJD YEAR DELTAT\n\
             60341 2024.2 69.5000\n\
             60310 2024.1 69.4000\n",
        )
        .unwrap_err();
        assert!(err.contains("predicted Delta T"));
        assert!(err.contains("not strictly increasing"));
    }

    #[test]
    fn parse_eop_finals_reads_representative_rows() {
        let text = format!(
            "{}\n{}\n",
            sample_eop_line(
                60_000,
                'I',
                -0.123_456_7,
                Some(0.123_456),
                Some(-0.234_567),
                Some(1.2345),
                Some(0.321),
                Some(-0.111),
            ),
            sample_eop_line(60_001, 'P', -0.223_456_7, None, None, None, None, None,),
        );
        let points = parse_eop_finals(&text).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].mjd, 60_000);
        assert!(points[0].ut1_observed);
        assert_eq!(points[0].pm_xp_arcsec, Some(0.123_456));
        assert_eq!(points[0].lod_milliseconds, Some(1.2345));
        assert_eq!(points[0].dx_milliarcsec, Some(0.321));
        assert_eq!(points[1].mjd, 60_001);
        assert!(!points[1].ut1_observed);
        assert_eq!(points[1].pm_xp_arcsec, None);
        assert_eq!(points[1].dx_milliarcsec, None);
    }

    #[test]
    fn parse_eop_finals_rejects_duplicate_mjds() {
        let text = format!(
            "{}\n{}\n",
            sample_eop_line(60_000, 'I', -0.1, Some(0.1), Some(0.2), None, None, None),
            sample_eop_line(60_000, 'P', -0.2, Some(0.1), Some(0.2), None, None, None),
        );
        let err = parse_eop_finals(&text).unwrap_err();
        assert!(err.contains("duplicate"));
    }

    #[test]
    fn parse_eop_finals_rejects_daily_gaps() {
        let text = format!(
            "{}\n{}\n",
            sample_eop_line(60_000, 'I', -0.1, Some(0.1), Some(0.2), None, None, None),
            sample_eop_line(60_002, 'P', -0.2, Some(0.1), Some(0.2), None, None, None),
        );
        let err = parse_eop_finals(&text).unwrap_err();
        assert!(err.contains("daily gap"));
    }

    // ── TimeDataError display, source and From ───────────────────────────────

    #[test]
    fn time_data_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = TimeDataError::Io(io_err);
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn time_data_error_display_download() {
        let err = TimeDataError::Download("timeout".into());
        assert!(err.to_string().contains("download error"));
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn time_data_error_display_parse() {
        let err = TimeDataError::Parse("bad line".into());
        assert!(err.to_string().contains("parse error"));
        assert!(err.to_string().contains("bad line"));
    }

    #[test]
    fn time_data_error_display_integrity() {
        let err = TimeDataError::Integrity("hash mismatch".into());
        assert!(err.to_string().contains("integrity error"));
        assert!(err.to_string().contains("hash mismatch"));
    }

    #[test]
    fn time_data_error_source_io_is_some() {
        use std::error::Error;
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = TimeDataError::Io(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn time_data_error_source_non_io_is_none() {
        use std::error::Error;
        assert!(TimeDataError::Download("x".into()).source().is_none());
        assert!(TimeDataError::Parse("x".into()).source().is_none());
        assert!(TimeDataError::Integrity("x".into()).source().is_none());
    }

    #[test]
    fn time_data_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe");
        let err = TimeDataError::from(io_err);
        assert!(matches!(err, TimeDataError::Io(_)));
    }

    // ── parse_month additional month names ───────────────────────────────────

    #[test]
    fn parse_utc_tai_segments_with_sep_oct_nov_dec() {
        // Test month names not exercised by the default sample fixture.
        let history = "\
1961 Sep. 1 - Oct. 1 0.5s\n\
Oct. 1 - Nov. 1 0.6s\n\
Nov. 1 - Dec. 1 0.7s\n\
Dec. 1 - 1962 Jan. 1 0.8s\n\
1962 Jan. 1 - 1.0s\n";
        let segments = parse_utc_tai_segments(history).unwrap();
        assert!(segments.len() >= 5);
    }

    // ── parse_date_fragment error path ───────────────────────────────────────

    #[test]
    fn parse_utc_tai_segments_rejects_bad_date_fragment() {
        // A line that has a dash but whose left side has alphabetic chars yet
        // cannot be parsed as a valid date fragment (too many tokens).
        let history = "baddate foo bar baz qux - 1962 Jan. 1 1.0s\n1962 Jan. 1 - 2.0s\n";
        let result = parse_utc_tai_segments(history);
        assert!(result.is_err());
    }

    // ── parse_utc_tai_segments with no-slope formula ─────────────────────────

    #[test]
    fn parse_utc_tai_segments_constant_offset_formula() {
        let history = "1972 Jan. 1 - 1973 Jan. 1 10s\n1973 Jan. 1 - 11s\n";
        let segments = parse_utc_tai_segments(history).unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].base_seconds, 10.0);
        assert_eq!(segments[0].slope_seconds_per_day, 0.0);
        assert_eq!(segments[1].base_seconds, 11.0);
        assert_eq!(segments[1].slope_seconds_per_day, 0.0);
        assert!(segments[1].end_mjd.is_none());
    }
}
