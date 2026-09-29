// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use crate::archive::time::{TimeDataBundle, UtcTaiSegment};
use crate::encoding::{
    day_to_j2000_seconds, j2000_seconds_to_day, jd_to_mjd, mjd_to_unix_seconds, unix_seconds_to_jd,
};
use crate::format::JD;
use crate::foundation::constats::{TT_MINUS_TAI, UTC_INTERVAL_EPS};
use crate::foundation::error::ConversionError;
use crate::qtty::{self, Day, Nanosecond, Second};
use chrono::{DateTime, Utc};

const NANOS_PER_SECOND: Nanosecond = Nanosecond::new(1_000_000_000.0);

#[derive(Clone, Copy)]
enum UtcTaiRegion {
    Segment(UtcTaiSegment),
    Leap {
        end_mjd: Day,
        end_tt: Day,
        next_start_tt: Day,
    },
}

/// Return TAI − UTC in seconds at the given UTC MJD.
///
/// Returns `Err(ConversionError::UtcBeforeDefinition)` for dates before
/// MJD 37 300 (1961-01-01) when `allow_extrapolation` is `false`. When
/// `true`, extrapolates the first official UTC-TAI segment backwards; the
/// result is internally consistent (round-trips close) but is not
/// historically defined UTC.
pub(crate) fn time_data_try_tai_minus_utc_mjd(
    data: &TimeDataBundle,
    mjd_utc: Day,
    allow_extrapolation: bool,
) -> Result<Second, ConversionError> {
    let segments = data.utc_tai_segments();
    let first = segments[0];
    if mjd_utc < Day::new(first.start_mjd as f64) {
        if !allow_extrapolation {
            return Err(ConversionError::UtcBeforeDefinition);
        }
        return Ok(utc_offset_seconds_in_segment(mjd_utc, first));
    }
    let idx = segments.partition_point(|segment| Day::new(segment.start_mjd as f64) <= mjd_utc);
    let segment = segments[idx - 1];
    Ok(utc_offset_seconds_in_segment(mjd_utc, segment))
}

/// Like [`time_data_try_tai_minus_utc_mjd`] but always extrapolates; used
/// for internal ΔT / EOP bookkeeping that must not surface the pre-definition
/// policy to callers.
pub(super) fn time_data_tai_minus_utc_mjd_extrapolated(
    data: &TimeDataBundle,
    mjd_utc: Day,
) -> Option<Second> {
    time_data_try_tai_minus_utc_mjd(data, mjd_utc, true).ok()
}

pub(crate) fn time_data_utc_from_tai_seconds(
    data: &TimeDataBundle,
    tai_secs: Second,
    allow_extrapolation: bool,
) -> Result<DateTime<Utc>, ConversionError> {
    if tai_secs.value().is_nan() {
        return Err(ConversionError::NonFinite);
    }
    let jd_tt = j2000_seconds_to_day::<JD>(tai_secs + TT_MINUS_TAI);
    let mjd_tt = jd_to_mjd(jd_tt);
    match locate_utc_region_from_tt_mjd(data.utc_tai_segments(), mjd_tt, allow_extrapolation)? {
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
            let leap_nanos: Nanosecond = NANOS_PER_SECOND
                + (mjd_tt - end_tt)
                    .to::<qtty::unit::Second>()
                    .to::<qtty::unit::Nanosecond>();
            let window_nanos: Nanosecond = (next_start_tt - end_tt)
                .to::<qtty::unit::Second>()
                .to::<qtty::unit::Nanosecond>()
                .round()
                .max(Nanosecond::one());
            let max_nanos = NANOS_PER_SECOND + window_nanos - Nanosecond::one();
            let nanos = leap_nanos.round().clamp(NANOS_PER_SECOND, max_nanos);
            DateTime::<Utc>::from_timestamp(base_secs, (nanos / Nanosecond::one()) as u32)
                .ok_or(ConversionError::OutOfRange)
        }
    }
}

pub(crate) fn time_data_tai_seconds_from_utc(
    data: &TimeDataBundle,
    dt: DateTime<Utc>,
    allow_extrapolation: bool,
) -> Result<Second, ConversionError> {
    let base_jd_utc = unix_seconds_to_jd(Second::new(dt.timestamp() as f64));
    let tai_minus_utc =
        time_data_try_tai_minus_utc_mjd(data, jd_to_mjd(base_jd_utc), allow_extrapolation)?;
    let subsec_nanos = dt.timestamp_subsec_nanos();
    if subsec_nanos >= 1_000_000_000 {
        let next = time_data_try_tai_minus_utc_mjd(
            data,
            jd_to_mjd(base_jd_utc) + Second::new(1.0).to::<qtty::unit::Day>(),
            allow_extrapolation,
        )
        .map_err(|_| ConversionError::InvalidLeapSecond)?;
        if next - tai_minus_utc < Second::new(0.5) {
            return Err(ConversionError::InvalidLeapSecond);
        }
    }

    let frac = Nanosecond::new(subsec_nanos as f64).to::<qtty::unit::Second>();
    Ok(day_to_j2000_seconds::<JD>(base_jd_utc) + tai_minus_utc + frac)
}

pub(crate) fn time_data_tai_seconds_is_in_leap_window(
    data: &TimeDataBundle,
    tai_secs: Second,
) -> bool {
    let jd_tt = j2000_seconds_to_day::<JD>(tai_secs + TT_MINUS_TAI);
    let mjd_tt = jd_to_mjd(jd_tt);
    // Pre-1961 times are never in a leap-second window; passing false is safe.
    matches!(
        locate_utc_region_from_tt_mjd(data.utc_tai_segments(), mjd_tt, false),
        Ok(UtcTaiRegion::Leap { .. })
    )
}

fn utc_offset_seconds_in_segment(mjd_utc: Day, segment: UtcTaiSegment) -> Second {
    let utc_offset = mjd_utc - Day::new(segment.reference_mjd);
    segment.base + Second::new(segment.slope_seconds_per_day) * (utc_offset / Day::new(1.0))
}

fn utc_mjd_to_tt_mjd_in_segment(mjd_utc: Day, segment: UtcTaiSegment) -> Day {
    mjd_utc
        + (utc_offset_seconds_in_segment(mjd_utc, segment) + TT_MINUS_TAI).to::<qtty::unit::Day>()
}

fn tt_mjd_to_utc_mjd_in_segment(mjd_tt: Day, segment: UtcTaiSegment) -> Day {
    let scale = Day::new(1.0) + Second::new(segment.slope_seconds_per_day).to::<qtty::unit::Day>();
    let ref_days = Day::new(segment.reference_mjd) / Day::new(1.0);
    let offset_days = (segment.base - Second::new(segment.slope_seconds_per_day) * ref_days
        + TT_MINUS_TAI)
        .to::<qtty::unit::Day>();
    Day::new((mjd_tt - offset_days) / scale)
}

fn segment_start_tt(segment: UtcTaiSegment) -> Day {
    utc_mjd_to_tt_mjd_in_segment(Day::new(segment.start_mjd as f64), segment)
}

fn locate_utc_region_from_tt_mjd(
    segments: &[UtcTaiSegment],
    mjd_tt: Day,
    allow_extrapolation: bool,
) -> Result<UtcTaiRegion, ConversionError> {
    let idx =
        segments.partition_point(|segment| segment_start_tt(*segment) <= mjd_tt + UTC_INTERVAL_EPS);
    if idx == 0 && !allow_extrapolation {
        return Err(ConversionError::UtcBeforeDefinition);
    }
    let segment = segments[idx.saturating_sub(1)];
    if let Some(end_mjd) = segment.end_mjd {
        let end_tt = utc_mjd_to_tt_mjd_in_segment(Day::new(end_mjd as f64), segment);
        if mjd_tt >= end_tt - UTC_INTERVAL_EPS {
            if let Some(next) = segments.get(idx).copied() {
                let next_start_tt = segment_start_tt(next);
                if mjd_tt < next_start_tt - UTC_INTERVAL_EPS {
                    return Ok(UtcTaiRegion::Leap {
                        end_mjd: Day::new(end_mjd as f64),
                        end_tt,
                        next_start_tt,
                    });
                }
            }
        }
    }

    Ok(UtcTaiRegion::Segment(segment))
}

fn datetime_from_seconds_since_epoch(seconds_since_epoch: Second) -> Option<DateTime<Utc>> {
    if !seconds_since_epoch.is_finite() {
        return None;
    }

    let mut secs = seconds_since_epoch.floor();
    let mut nanos: Nanosecond = (seconds_since_epoch - secs)
        .to::<qtty::unit::Nanosecond>()
        .round();
    if nanos >= NANOS_PER_SECOND {
        secs += Second::one();
        nanos -= NANOS_PER_SECOND;
    }

    DateTime::<Utc>::from_timestamp(
        (secs / Second::one()) as i64,
        (nanos / Nanosecond::one()) as u32,
    )
}

fn datetime_from_utc_mjd(mjd_utc: Day) -> Option<DateTime<Utc>> {
    datetime_from_seconds_since_epoch(mjd_to_unix_seconds(mjd_utc))
}
