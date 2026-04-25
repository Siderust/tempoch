// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use crate::constats::{TT_MINUS_TAI, UTC_INTERVAL_EPS};
use crate::delta_t::delta_t_seconds_from_modern_points;
use crate::encoding::{
    j2000_seconds_to_jd, jd_to_j2000_seconds, jd_to_mjd, mjd_to_unix_seconds, unix_seconds_to_jd,
};
use crate::eop::EopValues;
use crate::error::ConversionError;
use crate::generated::eop_data::EOP_POINTS;
use crate::generated::time_data::{MODERN_DELTA_T_POINTS, UTC_TAI_SEGMENTS};
use chrono::{DateTime, Utc};
use qtty::time::{Days, Nanoseconds, Seconds};
use qtty::unit::{Day, Nanosecond, Second as SecondUnit};
use qtty::{Day as DayQuantity, Second};
use std::sync::{Arc, OnceLock, RwLock};
#[cfg(any(test, feature = "runtime-data-fetch"))]
use tempoch_time_data::TimeDataError as InternalDataError;
#[cfg(feature = "runtime-data-fetch")]
use tempoch_time_data::TimeDataManager;
use tempoch_time_data::{EopPoint, TimeDataBundle, TimeDataProvenance, UtcTaiSegment};

#[cfg(test)]
use std::sync::Mutex;

const NANOS_PER_SECOND: Nanoseconds = Nanoseconds::new(1_000_000_000.0);
#[cfg(test)]
const RUNTIME_DATA_MAX_AGE_SECONDS: i64 = 24 * 60 * 60;

#[derive(Clone, Copy)]
enum UtcTaiRegion {
    Segment(UtcTaiSegment),
    Leap {
        end_mjd: Days,
        end_tt: Days,
        next_start_tt: Days,
    },
}

static COMPILED_TIME_DATA: OnceLock<Arc<TimeDataBundle>> = OnceLock::new();
static ACTIVE_TIME_DATA: OnceLock<RwLock<Arc<TimeDataBundle>>> = OnceLock::new();

#[cfg(test)]
static TEST_TIME_DATA_GUARD: Mutex<()> = Mutex::new(());
#[cfg(test)]
static TEST_TIME_DATA: Mutex<Option<Arc<TimeDataBundle>>> = Mutex::new(None);

fn active_time_data_slot() -> &'static RwLock<Arc<TimeDataBundle>> {
    ACTIVE_TIME_DATA.get_or_init(|| RwLock::new(compiled_time_data()))
}

#[cfg(any(test, feature = "runtime-data-fetch"))]
fn set_active_time_data(bundle: TimeDataBundle) {
    let mut slot = active_time_data_slot()
        .write()
        .unwrap_or_else(|err| err.into_inner());
    *slot = Arc::new(bundle);
}

pub(crate) fn active_time_data() -> Arc<TimeDataBundle> {
    #[cfg(test)]
    if let Some(bundle) = TEST_TIME_DATA
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .clone()
    {
        return bundle;
    }

    active_time_data_slot()
        .read()
        .unwrap_or_else(|err| err.into_inner())
        .clone()
}

/// Load runtime time data into the active bundle.
///
/// This is cache-first: it uses the current cached bundle if present,
/// falling back to a refresh when no valid cache is available.
#[cfg(feature = "runtime-data-fetch")]
pub fn update_runtime_time_data() -> Result<(), crate::error::TimeDataError> {
    load_and_activate_runtime_time_data(false).map_err(Into::into)
}

/// Force-refresh runtime time data and load it into the active bundle.
#[cfg(feature = "runtime-data-fetch")]
pub fn refresh_runtime_time_data() -> Result<(), crate::error::TimeDataError> {
    load_and_activate_runtime_time_data(true).map_err(Into::into)
}

/// Explicitly fetch the latest runtime time data and load it into the active
/// bundle.
#[cfg(feature = "runtime-data-fetch")]
pub fn fetch_latest_time_data() -> Result<(), crate::error::TimeDataError> {
    refresh_runtime_time_data()
}

#[cfg(feature = "runtime-data-fetch")]
fn load_and_activate_runtime_time_data(force_refresh: bool) -> Result<(), InternalDataError> {
    let manager = TimeDataManager::new()?;
    let bundle = select_time_data(
        manager.load_cached(),
        || manager.refresh_and_load(),
        force_refresh,
    )?;
    set_active_time_data(bundle);
    Ok(())
}

#[cfg(any(test, feature = "runtime-data-fetch"))]
fn select_time_data(
    cached: Result<TimeDataBundle, InternalDataError>,
    refresh: impl FnOnce() -> Result<TimeDataBundle, InternalDataError>,
    force_refresh: bool,
) -> Result<TimeDataBundle, InternalDataError> {
    if force_refresh {
        return refresh();
    }

    match cached {
        Ok(bundle) => Ok(bundle),
        Err(_) => refresh(),
    }
}

#[cfg(test)]
fn bundle_is_stale(bundle: &TimeDataBundle, now: DateTime<Utc>) -> bool {
    match bundle.provenance().fetched_at() {
        Some(fetched_at) => {
            now.signed_duration_since(fetched_at).num_seconds() > RUNTIME_DATA_MAX_AGE_SECONDS
        }
        None => true,
    }
}

#[cfg(test)]
fn select_time_data_for_auto_refresh(
    cached: Result<TimeDataBundle, InternalDataError>,
    refresh: impl FnOnce() -> Result<TimeDataBundle, InternalDataError>,
    now: DateTime<Utc>,
) -> Result<TimeDataBundle, InternalDataError> {
    match cached {
        Ok(bundle) if !bundle_is_stale(&bundle, now) => Ok(bundle),
        Ok(bundle) => refresh().or(Ok(bundle)),
        Err(_) => refresh(),
    }
}

pub(crate) fn time_data_delta_t(
    data: &TimeDataBundle,
    jd_ut: DayQuantity,
) -> Result<Second, ConversionError> {
    delta_t_seconds_from_modern_points(jd_ut, data.modern_delta_t_points())
}

pub(crate) fn time_data_eop_at(data: &TimeDataBundle, mjd_utc: DayQuantity) -> Option<EopValues> {
    let points = data.eop_points();
    let first = points.first()?.mjd;
    let last = points.last()?.mjd;
    let mjd_f = mjd_utc.value();
    let lo_i = mjd_f.floor() as i32;
    let hi_i = lo_i + 1;
    if lo_i < first || lo_i > last {
        return None;
    }
    let lo = find_eop_point(points, lo_i)?;
    let hi = if hi_i > last {
        lo
    } else {
        find_eop_point(points, hi_i)?
    };

    let frac = if lo.mjd == hi.mjd {
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

    let ut1_minus_utc = {
        let lo_offset = time_data_try_tai_minus_utc_mjd(data, DayQuantity::new(lo_i as f64));
        let hi_offset = time_data_try_tai_minus_utc_mjd(data, DayQuantity::new(hi_i as f64));
        let query_offset = time_data_try_tai_minus_utc_mjd(data, mjd_utc);
        match (lo_offset, hi_offset, query_offset) {
            (Some(lo_tmu), Some(hi_tmu), Some(query_tmu)) => {
                let lo_cont = lo.ut1_minus_utc_seconds - lo_tmu.value();
                let hi_cont = hi.ut1_minus_utc_seconds - hi_tmu.value();
                Second::new(lerp(lo_cont, hi_cont) + query_tmu.value())
            }
            _ => Second::new(lerp(lo.ut1_minus_utc_seconds, hi.ut1_minus_utc_seconds)),
        }
    };

    Some(EopValues {
        mjd_utc,
        pm_xp_arcsec: lerp_opt(lo.pm_xp_arcsec, hi.pm_xp_arcsec),
        pm_yp_arcsec: lerp_opt(lo.pm_yp_arcsec, hi.pm_yp_arcsec),
        ut1_minus_utc,
        lod_milliseconds,
        dx_milliarcsec: lerp_opt(lo.dx_milliarcsec, hi.dx_milliarcsec),
        dy_milliarcsec: lerp_opt(lo.dy_milliarcsec, hi.dy_milliarcsec),
        ut1_observed: lo.ut1_observed && hi.ut1_observed,
    })
}

fn find_eop_point(points: &[EopPoint], mjd: i32) -> Option<EopPoint> {
    let idx = points.partition_point(|point| point.mjd < mjd);
    let point = *points.get(idx)?;
    (point.mjd == mjd).then_some(point)
}

/// Return TAI − UTC in seconds at the given UTC date as a Modified Julian Day.
///
/// # Pre-history extrapolation
///
/// The UTC-TAI segment table starts at MJD 37 300 (1961-01-01). For any query
/// date **before** that boundary, this function extrapolates the first table
/// segment backwards and returns `Some(offset)`. The extrapolated value is
/// internally consistent — a UTC↔TAI round-trip will close — but it does not
/// represent a historically defined UTC-TAI value. UTC as an international
/// standard was not defined before 1961.
///
/// Callers that require historically accurate UTC values should compare the
/// query MJD against [`crate::constats::UTC_DEFINED_FROM_MJD`] and reject
/// or warn on pre-history dates themselves.
pub(crate) fn time_data_try_tai_minus_utc_mjd(
    data: &TimeDataBundle,
    mjd_utc: Days,
) -> Option<Seconds> {
    let segments = data.utc_tai_segments();
    let first = segments[0];
    if mjd_utc < DayQuantity::new(first.start_mjd as f64) {
        return Some(utc_offset_seconds_in_segment(mjd_utc, first));
    }
    let idx =
        segments.partition_point(|segment| DayQuantity::new(segment.start_mjd as f64) <= mjd_utc);
    let segment = segments[idx - 1];
    Some(utc_offset_seconds_in_segment(mjd_utc, segment))
}

pub(crate) fn time_data_utc_from_tai_seconds(
    data: &TimeDataBundle,
    tai_secs: Seconds,
) -> Result<DateTime<Utc>, ConversionError> {
    if !tai_secs.is_finite() {
        return Err(ConversionError::NonFinite);
    }
    let jd_tt = j2000_seconds_to_jd(tai_secs + TT_MINUS_TAI);
    let mjd_tt = jd_to_mjd(jd_tt);
    match locate_utc_region_from_tt_mjd(data.utc_tai_segments(), mjd_tt)? {
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

pub(crate) fn time_data_tai_seconds_from_utc(
    data: &TimeDataBundle,
    dt: DateTime<Utc>,
) -> Result<Second, ConversionError> {
    let base_jd_utc = unix_seconds_to_jd(Seconds::new(dt.timestamp() as f64));
    let tai_minus_utc = time_data_try_tai_minus_utc_mjd(data, jd_to_mjd(base_jd_utc))
        .ok_or(ConversionError::UtcHistoryUnsupported)?;
    let subsec_nanos = dt.timestamp_subsec_nanos();
    if subsec_nanos >= 1_000_000_000 {
        let next = time_data_try_tai_minus_utc_mjd(
            data,
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

pub(crate) fn time_data_tai_seconds_is_in_leap_window(
    data: &TimeDataBundle,
    tai_secs: Second,
) -> bool {
    let jd_tt = j2000_seconds_to_jd(tai_secs + TT_MINUS_TAI);
    let mjd_tt = jd_to_mjd(jd_tt);
    matches!(
        locate_utc_region_from_tt_mjd(data.utc_tai_segments(), mjd_tt),
        Ok(UtcTaiRegion::Leap { .. })
    )
}

#[cfg(test)]
pub(crate) fn with_test_time_data<T>(data: TimeDataBundle, f: impl FnOnce() -> T) -> T {
    let _guard = TEST_TIME_DATA_GUARD
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let mut slot = TEST_TIME_DATA.lock().unwrap_or_else(|err| err.into_inner());
    let previous = slot.replace(Arc::new(data));
    drop(slot);
    let result = f();
    *TEST_TIME_DATA.lock().unwrap_or_else(|err| err.into_inner()) = previous;
    result
}

fn compiled_time_data() -> Arc<TimeDataBundle> {
    COMPILED_TIME_DATA
        .get_or_init(|| {
            Arc::new(TimeDataBundle::new(
                UTC_TAI_SEGMENTS
                    .iter()
                    .map(|segment| UtcTaiSegment {
                        start_mjd: segment.start_mjd,
                        end_mjd: segment.end_mjd,
                        base_seconds: segment.base_seconds,
                        reference_mjd: segment.reference_mjd,
                        slope_seconds_per_day: segment.slope_seconds_per_day,
                    })
                    .collect(),
                MODERN_DELTA_T_POINTS.to_vec(),
                crate::MODERN_DELTA_T_OBSERVED_END_MJD.value(),
                EOP_POINTS
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
                    .collect(),
                TimeDataProvenance::new("compiled", "compiled", "compiled", "compiled", "compiled"),
            ))
        })
        .clone()
}

fn utc_offset_seconds_in_segment(mjd_utc: Days, segment: UtcTaiSegment) -> Seconds {
    let utc_offset = mjd_utc - DayQuantity::new(segment.reference_mjd);
    Second::new(segment.base_seconds)
        + Second::new(segment.slope_seconds_per_day) * (utc_offset / DayQuantity::new(1.0))
}

fn utc_mjd_to_tt_mjd_in_segment(mjd_utc: Days, segment: UtcTaiSegment) -> Days {
    mjd_utc + (utc_offset_seconds_in_segment(mjd_utc, segment) + TT_MINUS_TAI).to::<Day>()
}

fn tt_mjd_to_utc_mjd_in_segment(mjd_tt: Days, segment: UtcTaiSegment) -> Days {
    let scale = Days::new(1.0) + Second::new(segment.slope_seconds_per_day).to::<Day>();
    let ref_days = DayQuantity::new(segment.reference_mjd) / Days::new(1.0);
    let offset_days = (Second::new(segment.base_seconds)
        - Second::new(segment.slope_seconds_per_day) * ref_days
        + TT_MINUS_TAI)
        .to::<Day>();
    Days::new((mjd_tt - offset_days) / scale)
}

fn segment_start_tt(segment: UtcTaiSegment) -> Days {
    utc_mjd_to_tt_mjd_in_segment(DayQuantity::new(segment.start_mjd as f64), segment)
}

fn locate_utc_region_from_tt_mjd(
    segments: &[UtcTaiSegment],
    mjd_tt: Days,
) -> Result<UtcTaiRegion, ConversionError> {
    let idx =
        segments.partition_point(|segment| segment_start_tt(*segment) <= mjd_tt + UTC_INTERVAL_EPS);
    let segment = segments[idx.saturating_sub(1)];
    if let Some(end_mjd) = segment.end_mjd {
        let end_tt = utc_mjd_to_tt_mjd_in_segment(DayQuantity::new(end_mjd as f64), segment);
        if mjd_tt >= end_tt - UTC_INTERVAL_EPS {
            if let Some(next) = segments.get(idx).copied() {
                let next_start_tt = segment_start_tt(next);
                if mjd_tt < next_start_tt - UTC_INTERVAL_EPS {
                    return Ok(UtcTaiRegion::Leap {
                        end_mjd: DayQuantity::new(end_mjd as f64),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Time, TimeContext, TT, UT1, UTC};
    use qtty::Second;
    use tempoch_time_data::TimeDataProvenance;

    fn compiled_bundle_owned() -> TimeDataBundle {
        (*compiled_time_data()).clone()
    }

    fn bundle_with_timestamp(timestamp: &str) -> TimeDataBundle {
        let bundle = compiled_bundle_owned();
        TimeDataBundle::new(
            bundle.utc_tai_segments().to_vec(),
            bundle.modern_delta_t_points().to_vec(),
            bundle.modern_delta_t_observed_end_mjd(),
            bundle.eop_points().to_vec(),
            TimeDataProvenance::new(timestamp, "a", "b", "c", "d"),
        )
    }

    #[test]
    fn cache_is_selected_when_not_forcing_refresh() {
        let cached = bundle_with_timestamp("cached");
        let selected = select_time_data(
            Ok(cached.clone()),
            || {
                Err(InternalDataError::Integrity(
                    "refresh should not be called".into(),
                ))
            },
            false,
        )
        .unwrap();
        assert_eq!(selected.provenance().fetched_utc(), "cached");
    }

    #[test]
    fn missing_cache_triggers_refresh() {
        let refreshed = bundle_with_timestamp("refreshed");
        let selected = select_time_data(
            Err(InternalDataError::Integrity("missing cache".into())),
            || Ok(refreshed.clone()),
            false,
        )
        .unwrap();
        assert_eq!(selected.provenance().fetched_utc(), "refreshed");
    }

    #[test]
    fn force_refresh_ignores_cache() {
        let cached = bundle_with_timestamp("cached");
        let refreshed = bundle_with_timestamp("refreshed");
        let selected = select_time_data(Ok(cached), || Ok(refreshed.clone()), true).unwrap();
        assert_eq!(selected.provenance().fetched_utc(), "refreshed");
    }

    #[test]
    fn force_refresh_propagates_refresh_error() {
        let err = select_time_data(
            Ok(bundle_with_timestamp("cached")),
            || Err(InternalDataError::Download("network unreachable".into())),
            true,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("network unreachable"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn stale_cache_prefers_refresh_but_falls_back_if_refresh_fails() {
        let stale = bundle_with_timestamp("2026-04-15T00:00:00");
        let now = DateTime::from_timestamp(1_776_134_400, 0).unwrap();
        let selected = select_time_data_for_auto_refresh(
            Ok(stale.clone()),
            || Err(InternalDataError::Download("network unreachable".into())),
            now,
        )
        .unwrap();
        assert_eq!(
            selected.provenance().fetched_utc(),
            stale.provenance().fetched_utc()
        );
    }

    #[test]
    fn fresh_cache_skips_refresh_in_auto_mode() {
        let fresh = bundle_with_timestamp("2026-04-20T00:00:00");
        let now = DateTime::from_timestamp(1_776_139_200, 0).unwrap();
        let selected = select_time_data_for_auto_refresh(
            Ok(fresh.clone()),
            || {
                Err(InternalDataError::Integrity(
                    "refresh should not be called".into(),
                ))
            },
            now,
        )
        .unwrap();
        assert_eq!(
            selected.provenance().fetched_utc(),
            fresh.provenance().fetched_utc()
        );
    }

    #[test]
    fn ordinary_ut1_api_uses_override_bundle() {
        let bundle = compiled_bundle_owned();
        let mut eop_points = bundle.eop_points().to_vec();
        let point = eop_points.iter().position(|p| p.mjd == 57_000).unwrap();
        eop_points[point].ut1_minus_utc_seconds += 0.5;
        let bundle = TimeDataBundle::new(
            bundle.utc_tai_segments().to_vec(),
            bundle.modern_delta_t_points().to_vec(),
            bundle.modern_delta_t_observed_end_mjd(),
            eop_points,
            bundle.provenance().clone(),
        );

        with_test_time_data(bundle, || {
            let ctx = TimeContext::with_builtin_eop();
            let tt =
                Time::<TT>::from_julian_days(DayQuantity::new(2_400_000.5 + 57_000.0)).unwrap();
            let compiled = {
                let data = compiled_time_data();
                let dut1 = time_data_eop_at(data.as_ref(), DayQuantity::new(57_000.0))
                    .unwrap()
                    .ut1_minus_utc;
                dut1
            };
            let overridden = ctx.ut1_minus_utc(DayQuantity::new(57_000.0)).unwrap();
            assert!((overridden - compiled).abs() > Second::new(0.1));

            let ut1: Time<UT1> = tt.to_scale_with::<UT1>(&ctx).unwrap();
            assert!(ut1.julian_days().is_finite());
        });
    }

    #[test]
    fn time_context_snapshots_ut1_data_across_active_bundle_updates() {
        let _guard = TEST_TIME_DATA_GUARD
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let previous = active_time_data();
        let baseline = compiled_bundle_owned();
        set_active_time_data(baseline.clone());
        let ctx_before = TimeContext::with_builtin_eop();

        let mut eop_points = baseline.eop_points().to_vec();
        let point = eop_points.iter().position(|p| p.mjd == 57_000).unwrap();
        eop_points[point].ut1_minus_utc_seconds += 0.5;
        let overridden = TimeDataBundle::new(
            baseline.utc_tai_segments().to_vec(),
            baseline.modern_delta_t_points().to_vec(),
            baseline.modern_delta_t_observed_end_mjd(),
            eop_points,
            baseline.provenance().clone(),
        );
        set_active_time_data(overridden);
        let ctx_after = TimeContext::with_builtin_eop();

        let before = ctx_before
            .ut1_minus_utc(DayQuantity::new(57_000.0))
            .unwrap();
        let after = ctx_after.ut1_minus_utc(DayQuantity::new(57_000.0)).unwrap();
        set_active_time_data((*previous).clone());

        assert!((after - before).abs() > Second::new(0.1));
    }

    #[test]
    fn ordinary_utc_api_uses_override_bundle() {
        let bundle = compiled_bundle_owned();
        let mut segments = bundle.utc_tai_segments().to_vec();
        let segment = segments
            .iter()
            .position(|segment| segment.start_mjd <= 60_000 && segment.end_mjd.is_none())
            .unwrap();
        segments[segment].base_seconds += 1.0;
        let bundle = TimeDataBundle::new(
            segments,
            bundle.modern_delta_t_points().to_vec(),
            bundle.modern_delta_t_observed_end_mjd(),
            bundle.eop_points().to_vec(),
            bundle.provenance().clone(),
        );
        let unix = Second::new(1_680_000_000.25);
        let compiled_value = Time::<UTC>::from_unix_seconds(unix)
            .unwrap()
            .raw_seconds_pair()
            .0
            .value()
            + Time::<UTC>::from_unix_seconds(unix)
                .unwrap()
                .raw_seconds_pair()
                .1
                .value();

        with_test_time_data(bundle, || {
            let overridden = Time::<UTC>::from_unix_seconds(unix).unwrap();
            let overridden_value =
                overridden.raw_seconds_pair().0.value() + overridden.raw_seconds_pair().1.value();
            assert!((overridden_value - compiled_value).abs() > 0.1);
            let roundtrip = overridden.unix_seconds().unwrap();
            assert!((roundtrip - unix).abs() < Second::new(1e-3));
            let chrono = overridden.try_to_chrono().unwrap();
            let from_chrono = Time::<UTC>::try_from_chrono(chrono).unwrap();
            let drift = ((from_chrono.raw_seconds_pair().0.value()
                + from_chrono.raw_seconds_pair().1.value())
                - overridden_value)
                .abs();
            assert!(drift < 1e-4, "chrono round-trip drift = {drift}");
        });
    }

    #[test]
    fn time_context_snapshots_utc_civil_data_across_active_bundle_updates() {
        let _guard = TEST_TIME_DATA_GUARD
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let previous = active_time_data();
        let baseline = compiled_bundle_owned();
        set_active_time_data(baseline.clone());
        let ctx_before = TimeContext::new();

        let mut segments = baseline.utc_tai_segments().to_vec();
        let segment = segments
            .iter()
            .position(|segment| segment.start_mjd <= 60_000 && segment.end_mjd.is_none())
            .unwrap();
        segments[segment].base_seconds += 1.0;
        let overridden = TimeDataBundle::new(
            segments,
            baseline.modern_delta_t_points().to_vec(),
            baseline.modern_delta_t_observed_end_mjd(),
            baseline.eop_points().to_vec(),
            baseline.provenance().clone(),
        );
        set_active_time_data(overridden);
        let ctx_after = TimeContext::new();

        let unix = Second::new(1_680_000_000.25);
        let before = Time::<UTC>::from_unix_seconds_with(unix, &ctx_before).unwrap();
        let after = Time::<UTC>::from_unix_seconds_with(unix, &ctx_after).unwrap();
        let before_value =
            before.raw_seconds_pair().0.value() + before.raw_seconds_pair().1.value();
        let after_value = after.raw_seconds_pair().0.value() + after.raw_seconds_pair().1.value();
        set_active_time_data((*previous).clone());

        assert!((after_value - before_value).abs() > 0.1);
    }

    #[test]
    fn pre_1961_utc_roundtrips_with_approximate_segment_extension() {
        let dt = DateTime::from_timestamp(-631_152_000, 250_000_000).unwrap();
        let utc = Time::<UTC>::try_from_chrono(dt).unwrap();
        let back = utc.try_to_chrono().unwrap();
        let drift = (back.timestamp_nanos_opt().unwrap() - dt.timestamp_nanos_opt().unwrap()).abs();
        assert!(drift < 50_000, "pre-1961 UTC round-trip drift = {drift} ns");

        let unix = Second::new(-631_152_000.75);
        let utc_from_unix = Time::<UTC>::from_unix_seconds(unix).unwrap();
        let unix_back = utc_from_unix.unix_seconds().unwrap();
        assert!((unix_back - unix).abs() < Second::new(1e-3));
    }

    #[test]
    fn runtime_bundle_can_extend_delta_t_horizon_through_existing_api() {
        let bundle = compiled_bundle_owned();
        let mut points = bundle.modern_delta_t_points().to_vec();
        let last = *points.last().unwrap();
        points.push((last.0 + 31.0, last.1 + 0.25));
        let bundle = TimeDataBundle::new(
            bundle.utc_tai_segments().to_vec(),
            points,
            bundle.modern_delta_t_observed_end_mjd(),
            bundle.eop_points().to_vec(),
            bundle.provenance().clone(),
        );
        let beyond = crate::DELTA_T_PREDICTION_HORIZON_MJD + DayQuantity::new(15.0);
        let jd = beyond + crate::constats::JD_MINUS_MJD;
        let tt = Time::<TT>::from_julian_days(jd).unwrap();

        assert_eq!(
            tt.to_scale_with::<UT1>(&TimeContext::new()).unwrap_err(),
            ConversionError::Ut1HorizonExceeded
        );

        with_test_time_data(bundle, || {
            let ut1 = tt.to_scale_with::<UT1>(&TimeContext::new()).unwrap();
            assert!(ut1.julian_days().is_finite());
        });
    }

    #[test]
    fn eop_lookup_returns_none_when_bundle_has_gap() {
        let bundle = compiled_bundle_owned();
        let mut eop_points = bundle.eop_points().to_vec();
        let gap_idx = eop_points
            .windows(2)
            .position(|window| window[1].mjd == window[0].mjd + 1)
            .expect("compiled EOP series should contain adjacent rows")
            + 1;
        let gap_after = eop_points[gap_idx - 1].mjd;
        eop_points.remove(gap_idx);
        let bundle = TimeDataBundle::new(
            bundle.utc_tai_segments().to_vec(),
            bundle.modern_delta_t_points().to_vec(),
            bundle.modern_delta_t_observed_end_mjd(),
            eop_points,
            bundle.provenance().clone(),
        );

        assert!(time_data_eop_at(&bundle, DayQuantity::new(gap_after as f64 + 0.5)).is_none());
        assert!(time_data_eop_at(&bundle, DayQuantity::new((gap_after + 1) as f64)).is_none());
    }

    #[test]
    fn compiled_bundle_is_available() {
        let bundle = compiled_time_data();
        assert!(!bundle.utc_tai_segments().is_empty());
        assert!(!bundle.modern_delta_t_points().is_empty());
        assert!(!bundle.eop_points().is_empty());
    }
}
