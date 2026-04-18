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
use std::sync::{Arc, OnceLock};
use tempoch_time_data::{EopPoint, TimeDataBundle, TimeDataProvenance, UtcTaiSegment};

#[cfg(feature = "runtime-data")]
use std::time::Duration as StdDuration;
#[cfg(feature = "runtime-data")]
use tempoch_time_data::TimeDataManager;

const NANOS_PER_SECOND: Nanoseconds = Nanoseconds::new(1_000_000_000.0);
#[cfg(feature = "runtime-data")]
const REFRESH_TTL: StdDuration = StdDuration::from_secs(24 * 60 * 60);

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
#[cfg(all(not(test), feature = "runtime-data"))]
static ACTIVE_TIME_DATA: OnceLock<Arc<TimeDataBundle>> = OnceLock::new();

#[cfg(all(test, feature = "runtime-data"))]
use std::sync::Mutex;

#[cfg(all(test, feature = "runtime-data"))]
static TEST_TIME_DATA_GUARD: Mutex<()> = Mutex::new(());
#[cfg(all(test, feature = "runtime-data"))]
static TEST_TIME_DATA: Mutex<Option<Arc<TimeDataBundle>>> = Mutex::new(None);

#[cfg(all(not(test), not(feature = "runtime-data")))]
pub(crate) fn active_time_data() -> Arc<TimeDataBundle> {
    compiled_time_data()
}

#[cfg(all(not(test), feature = "runtime-data"))]
pub(crate) fn active_time_data() -> Arc<TimeDataBundle> {
    ACTIVE_TIME_DATA
        .get_or_init(|| match TimeDataManager::new() {
            Ok(manager) => Arc::new(resolve_time_data_with(&manager, Utc::now())),
            Err(_) => compiled_time_data(),
        })
        .clone()
}

#[cfg(all(test, not(feature = "runtime-data")))]
pub(crate) fn active_time_data() -> Arc<TimeDataBundle> {
    compiled_time_data()
}

#[cfg(all(test, feature = "runtime-data"))]
pub(crate) fn active_time_data() -> Arc<TimeDataBundle> {
    if let Some(bundle) = TEST_TIME_DATA
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .clone()
    {
        bundle
    } else {
        compiled_time_data()
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

pub(crate) fn time_data_try_tai_minus_utc_mjd(
    data: &TimeDataBundle,
    mjd_utc: Days,
) -> Option<Seconds> {
    let segments = data.utc_tai_segments();
    if mjd_utc < DayQuantity::new(segments[0].start_mjd as f64) {
        return None;
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

#[cfg(all(test, feature = "runtime-data"))]
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

#[cfg(all(not(test), feature = "runtime-data"))]
fn resolve_time_data_with(manager: &TimeDataManager, now: DateTime<Utc>) -> TimeDataBundle {
    let cached = manager.load_cached().ok();
    select_time_data(cached, || manager.refresh_and_load().ok(), now)
}

#[cfg(feature = "runtime-data")]
fn bundle_is_fresh(data: &TimeDataBundle, now: DateTime<Utc>) -> bool {
    let Some(fetched_at) = data.provenance().fetched_at() else {
        return false;
    };
    match now.signed_duration_since(fetched_at).to_std() {
        Ok(age) => age < REFRESH_TTL,
        Err(_) => true,
    }
}

#[cfg(feature = "runtime-data")]
fn select_time_data(
    cached: Option<TimeDataBundle>,
    refresh: impl FnOnce() -> Option<TimeDataBundle>,
    now: DateTime<Utc>,
) -> TimeDataBundle {
    if cached
        .as_ref()
        .is_some_and(|bundle| bundle_is_fresh(bundle, now))
    {
        return cached.unwrap();
    }

    refresh()
        .or(cached)
        .unwrap_or_else(|| (*compiled_time_data()).clone())
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
    let first = segments[0];
    if mjd_tt < segment_start_tt(first) - UTC_INTERVAL_EPS {
        return Err(ConversionError::UtcHistoryUnsupported);
    }

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
    #[cfg(feature = "runtime-data")]
    use crate::{Time, TimeContext, JD, TT, UT1, UTC};
    #[cfg(feature = "runtime-data")]
    use qtty::Second;
    #[cfg(feature = "runtime-data")]
    use tempoch_time_data::TimeDataProvenance;

    #[cfg(feature = "runtime-data")]
    fn compiled_bundle_owned() -> TimeDataBundle {
        (*compiled_time_data()).clone()
    }

    #[cfg(feature = "runtime-data")]
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

    #[cfg(feature = "runtime-data")]
    #[test]
    fn cache_freshness_uses_24_hour_ttl() {
        let now = DateTime::parse_from_rfc3339("2026-04-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(bundle_is_fresh(
            &bundle_with_timestamp("2026-04-17T12:01:00"),
            now
        ));
        assert!(!bundle_is_fresh(
            &bundle_with_timestamp("2026-04-17T11:59:59"),
            now
        ));
        assert!(!bundle_is_fresh(&bundle_with_timestamp("compiled"), now));
    }

    #[cfg(feature = "runtime-data")]
    #[test]
    fn fresh_cache_skips_refresh_and_wins() {
        let now = DateTime::parse_from_rfc3339("2026-04-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cached = bundle_with_timestamp("2026-04-18T11:30:00");
        let selected = select_time_data(Some(cached.clone()), || None, now);
        assert_eq!(
            selected.provenance().fetched_utc(),
            cached.provenance().fetched_utc()
        );
    }

    #[cfg(feature = "runtime-data")]
    #[test]
    fn stale_cache_prefers_successful_refresh() {
        let now = DateTime::parse_from_rfc3339("2026-04-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cached = bundle_with_timestamp("2026-04-16T12:00:00");
        let refreshed = bundle_with_timestamp("2026-04-18T11:59:00");
        let selected = select_time_data(Some(cached), || Some(refreshed.clone()), now);
        assert_eq!(
            selected.provenance().fetched_utc(),
            refreshed.provenance().fetched_utc()
        );
    }

    #[cfg(feature = "runtime-data")]
    #[test]
    fn stale_cache_survives_failed_refresh() {
        let now = DateTime::parse_from_rfc3339("2026-04-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let cached = bundle_with_timestamp("2026-04-16T12:00:00");
        let selected = select_time_data(Some(cached.clone()), || None, now);
        assert_eq!(
            selected.provenance().fetched_utc(),
            cached.provenance().fetched_utc()
        );
    }

    #[cfg(feature = "runtime-data")]
    #[test]
    fn missing_cache_and_failed_refresh_fall_back_to_compiled() {
        let now = DateTime::parse_from_rfc3339("2026-04-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let selected = select_time_data(None, || None, now);
        assert_eq!(selected.provenance().fetched_utc(), "compiled");
    }

    #[cfg(feature = "runtime-data")]
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
                Time::<TT, JD>::from_julian_days(DayQuantity::new(2_400_000.5 + 57_000.0)).unwrap();
            let compiled = {
                let data = compiled_time_data();
                let dut1 = time_data_eop_at(data.as_ref(), DayQuantity::new(57_000.0))
                    .unwrap()
                    .ut1_minus_utc;
                dut1
            };
            let overridden = ctx.ut1_minus_utc(DayQuantity::new(57_000.0)).unwrap();
            assert!((overridden - compiled).abs() > Second::new(0.1));

            let ut1: Time<UT1, JD> = tt.to_scale_with::<UT1>(&ctx).unwrap();
            assert!(ut1.julian_days().is_finite());
        });
    }

    #[cfg(feature = "runtime-data")]
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
            .value()
            .value();

        with_test_time_data(bundle, || {
            let overridden = Time::<UTC>::from_unix_seconds(unix).unwrap();
            assert!((overridden.value().value() - compiled_value).abs() > 0.1);
            let roundtrip = overridden.unix_seconds().unwrap();
            assert!((roundtrip - unix).abs() < Second::new(1e-3));
            let chrono = overridden.try_to_chrono().unwrap();
            let from_chrono = Time::<UTC>::try_from_chrono(chrono).unwrap();
            let drift = (from_chrono.value().value() - overridden.value().value()).abs();
            assert!(drift < 1e-4, "chrono round-trip drift = {drift}");
        });
    }

    #[cfg(feature = "runtime-data")]
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
        let tt = Time::<TT, JD>::from_julian_days(jd).unwrap();

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
    fn compiled_bundle_is_available() {
        let bundle = compiled_time_data();
        assert!(!bundle.utc_tai_segments().is_empty());
        assert!(!bundle.modern_delta_t_points().is_empty());
        assert!(!bundle.eop_points().is_empty());
    }
}
