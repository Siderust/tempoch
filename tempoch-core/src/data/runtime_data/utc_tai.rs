// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use crate::archive::time::{TimeDataBundle, UtcTaiSegment};
use crate::encoding::{
    day_to_j2000_seconds, j2000_seconds_to_day, jd_to_mjd, mjd_to_unix_seconds, unix_seconds_to_jd,
};
use crate::format::JD;
use crate::foundation::constats::{TT_MINUS_TAI, UTC_INTERVAL_EPS};
use crate::foundation::error::ConversionError;
use chrono::{DateTime, Utc};
use qtty::unit::{Day, Nanosecond, Second as SecondUnit};
use qtty::{Day as DayQuantity, Nanosecond as NanosecondQty, Second};

const NANOS_PER_SECOND: NanosecondQty = NanosecondQty::new(1_000_000_000.0);

#[derive(Clone, Copy)]
enum UtcTaiRegion {
    Segment(UtcTaiSegment),
    Leap {
        end_mjd: DayQuantity,
        end_tt: DayQuantity,
        next_start_tt: DayQuantity,
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
    mjd_utc: DayQuantity,
    allow_extrapolation: bool,
) -> Result<Second, ConversionError> {
    let segments = data.utc_tai_segments();
    let first = segments[0];
    if mjd_utc < DayQuantity::new(first.start_mjd as f64) {
        if !allow_extrapolation {
            return Err(ConversionError::UtcBeforeDefinition);
        }
        return Ok(utc_offset_seconds_in_segment(mjd_utc, first));
    }
    let idx =
        segments.partition_point(|segment| DayQuantity::new(segment.start_mjd as f64) <= mjd_utc);
    let segment = segments[idx - 1];
    Ok(utc_offset_seconds_in_segment(mjd_utc, segment))
}

/// Like [`time_data_try_tai_minus_utc_mjd`] but always extrapolates; used
/// for internal ΔT / EOP bookkeeping that must not surface the pre-definition
/// policy to callers.
pub(super) fn time_data_tai_minus_utc_mjd_extrapolated(
    data: &TimeDataBundle,
    mjd_utc: DayQuantity,
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
            let leap_nanos: NanosecondQty =
                NANOS_PER_SECOND + (mjd_tt - end_tt).to::<SecondUnit>().to::<Nanosecond>();
            let window_nanos: NanosecondQty = (next_start_tt - end_tt)
                .to::<SecondUnit>()
                .to::<Nanosecond>()
                .round()
                .max(NanosecondQty::one());
            let max_nanos = NANOS_PER_SECOND + window_nanos - NanosecondQty::one();
            let nanos = leap_nanos.round().clamp(NANOS_PER_SECOND, max_nanos);
            DateTime::<Utc>::from_timestamp(base_secs, (nanos / NanosecondQty::one()) as u32)
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
            jd_to_mjd(base_jd_utc) + Second::new(1.0).to::<Day>(),
            allow_extrapolation,
        )
        .map_err(|_| ConversionError::InvalidLeapSecond)?;
        if next - tai_minus_utc < Second::new(0.5) {
            return Err(ConversionError::InvalidLeapSecond);
        }
    }

    let frac = NanosecondQty::new(subsec_nanos as f64).to::<SecondUnit>();
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

fn utc_offset_seconds_in_segment(mjd_utc: DayQuantity, segment: UtcTaiSegment) -> Second {
    let utc_offset = mjd_utc - DayQuantity::new(segment.reference_mjd);
    segment.base + Second::new(segment.slope_seconds_per_day) * (utc_offset / DayQuantity::new(1.0))
}

fn utc_mjd_to_tt_mjd_in_segment(mjd_utc: DayQuantity, segment: UtcTaiSegment) -> DayQuantity {
    mjd_utc + (utc_offset_seconds_in_segment(mjd_utc, segment) + TT_MINUS_TAI).to::<Day>()
}

fn tt_mjd_to_utc_mjd_in_segment(mjd_tt: DayQuantity, segment: UtcTaiSegment) -> DayQuantity {
    let scale = DayQuantity::new(1.0) + Second::new(segment.slope_seconds_per_day).to::<Day>();
    let ref_days = DayQuantity::new(segment.reference_mjd) / DayQuantity::new(1.0);
    let offset_days = (segment.base - Second::new(segment.slope_seconds_per_day) * ref_days
        + TT_MINUS_TAI)
        .to::<Day>();
    DayQuantity::new((mjd_tt - offset_days) / scale)
}

fn segment_start_tt(segment: UtcTaiSegment) -> DayQuantity {
    utc_mjd_to_tt_mjd_in_segment(DayQuantity::new(segment.start_mjd as f64), segment)
}

fn locate_utc_region_from_tt_mjd(
    segments: &[UtcTaiSegment],
    mjd_tt: DayQuantity,
    allow_extrapolation: bool,
) -> Result<UtcTaiRegion, ConversionError> {
    let idx =
        segments.partition_point(|segment| segment_start_tt(*segment) <= mjd_tt + UTC_INTERVAL_EPS);
    if idx == 0 && !allow_extrapolation {
        return Err(ConversionError::UtcBeforeDefinition);
    }
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

fn datetime_from_seconds_since_epoch(seconds_since_epoch: Second) -> Option<DateTime<Utc>> {
    if !seconds_since_epoch.is_finite() {
        return None;
    }

    let mut secs = seconds_since_epoch.floor();
    let mut nanos: NanosecondQty = (seconds_since_epoch - secs).to::<Nanosecond>().round();
    if nanos >= NANOS_PER_SECOND {
        secs += Second::one();
        nanos -= NANOS_PER_SECOND;
    }

    DateTime::<Utc>::from_timestamp(
        (secs / Second::one()) as i64,
        (nanos / NanosecondQty::one()) as u32,
    )
}

fn datetime_from_utc_mjd(mjd_utc: DayQuantity) -> Option<DateTime<Utc>> {
    datetime_from_seconds_since_epoch(mjd_to_unix_seconds(mjd_utc))
}
