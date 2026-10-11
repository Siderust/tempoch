// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use crate::archive::time::{TimeDataBundle, UtcTaiSegment};
use crate::encoding::{day_to_j2000_seconds, jd_to_mjd, mjd_to_unix_seconds, unix_seconds_to_jd};
use crate::format::MJD;
use crate::foundation::constats::unix_epoch_mjd_day;
use crate::foundation::error::ConversionError;
use crate::qtty::{self, Day, Nanosecond, Second};
use affn::algebra::SplitQuantity;
use chrono::{DateTime, Utc};

type SecondsSplit = SplitQuantity<qtty::unit::Second>;
const NANOS_PER_SECOND: Nanosecond = Nanosecond::new(1_000_000_000.0);

#[derive(Clone, Copy)]
enum UtcTaiRegion {
    Segment(UtcTaiSegment),
    Leap {
        end_mjd: Day,
        start: SecondsSplit,
        end: SecondsSplit,
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

/// Context-owned TAI-UTC at an atomic instant, including a leap-second label.
pub(crate) fn time_data_tai_minus_utc_at(
    data: &TimeDataBundle,
    tai_pair: (Second, Second),
    allow_extrapolation: bool,
) -> Result<Second, ConversionError> {
    let tai = checked_seconds(tai_pair)?;
    match locate_utc_region(data.utc_tai_segments(), tai, allow_extrapolation)? {
        UtcTaiRegion::Segment(segment) => Ok(tai.difference(utc_seconds_in_segment(tai, segment))),
        UtcTaiRegion::Leap { end_mjd, start, .. } => Ok(start.difference(
            SecondsSplit::from_quantity(day_to_j2000_seconds::<MJD>(end_mjd)),
        )),
    }
}

pub(crate) fn time_data_utc_from_tai_seconds(
    data: &TimeDataBundle,
    tai_pair: (Second, Second),
    allow_extrapolation: bool,
) -> Result<DateTime<Utc>, ConversionError> {
    let tai = checked_seconds(tai_pair)?;
    match locate_utc_region(data.utc_tai_segments(), tai, allow_extrapolation)? {
        UtcTaiRegion::Segment(segment) => {
            let unix = shift(utc_seconds_in_segment(tai, segment), -unix_origin());
            datetime_from_seconds_since_epoch(unix).ok_or(ConversionError::OutOfRange)
        }
        UtcTaiRegion::Leap {
            end_mjd,
            start,
            end,
        } => {
            let boundary = mjd_to_unix_seconds(end_mjd);
            let leap_nanos =
                NANOS_PER_SECOND + tai.difference(start).to::<qtty::unit::Nanosecond>();
            let window_nanos = end
                .difference(start)
                .to::<qtty::unit::Nanosecond>()
                .round()
                .max(Nanosecond::one());
            let nanos = leap_nanos.round().clamp(
                NANOS_PER_SECOND,
                NANOS_PER_SECOND + window_nanos - Nanosecond::one(),
            );
            DateTime::<Utc>::from_timestamp(
                (boundary / Second::one()) as i64 - 1,
                (nanos / Nanosecond::one()) as u32,
            )
            .ok_or(ConversionError::OutOfRange)
        }
    }
}

pub(crate) fn time_data_tai_seconds_from_utc(
    data: &TimeDataBundle,
    dt: DateTime<Utc>,
    allow_extrapolation: bool,
) -> Result<(Second, Second), ConversionError> {
    let unix_whole = Second::new(dt.timestamp() as f64);
    // Scalar days are only used to select the UTC history segment. The
    // instant itself is assembled from integer seconds and fractional seconds.
    let base_mjd = jd_to_mjd(unix_seconds_to_jd(unix_whole));
    let segments = data.utc_tai_segments();
    let idx = segments.partition_point(|segment| Day::new(segment.start_mjd as f64) <= base_mjd);
    if idx == 0 && !allow_extrapolation {
        return Err(ConversionError::UtcBeforeDefinition);
    }
    let segment = segments[idx.saturating_sub(1)];
    let offset = utc_offset_seconds_in_segment(base_mjd, segment);
    let nanos = dt.timestamp_subsec_nanos();
    if nanos >= 1_000_000_000 {
        let next_mjd = crate::encoding::unix_seconds_to_mjd(unix_whole + Second::one());
        let next = time_data_try_tai_minus_utc_mjd(data, next_mjd, allow_extrapolation)
            .map_err(|_| ConversionError::InvalidLeapSecond)?;
        if next - offset < Second::new(0.5) {
            return Err(ConversionError::InvalidLeapSecond);
        }
    }
    let fraction = Nanosecond::new(nanos as f64).to::<qtty::unit::Second>();
    let drift = Second::new(segment.slope_seconds_per_day)
        * (fraction.to::<qtty::unit::Day>() / Day::one());
    Ok(SecondsSplit::new(unix_whole + unix_origin(), offset)
        .add_quantity(fraction)
        .add_quantity(drift)
        .pair())
}

pub(crate) fn time_data_tai_seconds_is_in_leap_window(
    data: &TimeDataBundle,
    tai_pair: (Second, Second),
) -> bool {
    checked_seconds(tai_pair).is_ok_and(|tai| {
        matches!(
            locate_utc_region(data.utc_tai_segments(), tai, false),
            Ok(UtcTaiRegion::Leap { .. })
        )
    })
}

fn checked_seconds(pair: (Second, Second)) -> Result<SecondsSplit, ConversionError> {
    if !pair.0.is_finite() || !pair.1.is_finite() {
        return Err(ConversionError::NonFinite);
    }
    Ok(SecondsSplit::new(pair.0, pair.1))
}

fn unix_origin() -> Second {
    day_to_j2000_seconds::<MJD>(unix_epoch_mjd_day())
}

fn utc_offset_seconds_in_segment(mjd_utc: Day, segment: UtcTaiSegment) -> Second {
    segment.base
        + Second::new(segment.slope_seconds_per_day)
            * ((mjd_utc - Day::new(segment.reference_mjd)) / Day::one())
}

fn tai_at_midnight(mjd: Day, segment: UtcTaiSegment) -> SecondsSplit {
    SecondsSplit::new(
        day_to_j2000_seconds::<MJD>(mjd),
        utc_offset_seconds_in_segment(mjd, segment),
    )
}

fn utc_seconds_in_segment(tai: SecondsSplit, segment: UtcTaiSegment) -> SecondsSplit {
    let reference = day_to_j2000_seconds::<MJD>(Day::new(segment.reference_mjd));
    let rate = 1.0
        + Second::new(segment.slope_seconds_per_day)
            .to::<qtty::unit::Day>()
            .value();
    shift(
        shift(tai, -reference)
            .sub_quantity(segment.base)
            .scale(1.0 / rate),
        reference,
    )
}

// Combine the large origins with the high word first. Adding an epoch-sized
// displacement to the low word would erase subsecond compensation.
fn shift(value: SecondsSplit, displacement: Second) -> SecondsSplit {
    let shifted = SecondsSplit::new(value.hi(), displacement);
    SecondsSplit::new(shifted.hi(), shifted.lo() + value.lo())
}

fn locate_utc_region(
    segments: &[UtcTaiSegment],
    tai: SecondsSplit,
    allow_extrapolation: bool,
) -> Result<UtcTaiRegion, ConversionError> {
    let start =
        |segment: UtcTaiSegment| tai_at_midnight(Day::new(segment.start_mjd as f64), segment);
    let idx =
        segments.partition_point(|segment| tai.difference(start(*segment)) >= Second::new(0.0));
    if idx == 0 && !allow_extrapolation {
        return Err(ConversionError::UtcBeforeDefinition);
    }
    let segment = segments[idx.saturating_sub(1)];
    if let (Some(end_mjd), Some(next)) = (segment.end_mjd, segments.get(idx).copied()) {
        let end_mjd = Day::new(end_mjd as f64);
        let leap_start = tai_at_midnight(end_mjd, segment);
        let leap_end = start(next);
        if tai.difference(leap_start) >= Second::new(0.0)
            && tai.difference(leap_end) < Second::new(0.0)
        {
            return Ok(UtcTaiRegion::Leap {
                end_mjd,
                start: leap_start,
                end: leap_end,
            });
        }
    }
    Ok(UtcTaiRegion::Segment(segment))
}

fn datetime_from_seconds_since_epoch(seconds: SecondsSplit) -> Option<DateTime<Utc>> {
    let mut whole = seconds.hi().floor();
    let mut fraction = seconds.difference(SecondsSplit::from_quantity(whole));
    if fraction < Second::new(0.0) {
        whole -= Second::one();
        fraction += Second::one();
    }
    let mut nanos = fraction.to::<qtty::unit::Nanosecond>().round();
    if nanos >= NANOS_PER_SECOND {
        whole += Second::one();
        nanos -= NANOS_PER_SECOND;
    }
    DateTime::<Utc>::from_timestamp(
        (whole / Second::one()) as i64,
        (nanos / Nanosecond::one()) as u32,
    )
}
