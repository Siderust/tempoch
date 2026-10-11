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
    let nanos = dt.timestamp_subsec_nanos();
    if nanos >= 1_000_000_000 {
        let boundary = unix_whole + unix_origin() + Second::one();
        if !segments.get(idx).is_some_and(|next| {
            is_positive_leap(segment, *next)
                && boundary == day_to_j2000_seconds::<MJD>(Day::new(next.start_mjd as f64))
        }) {
            return Err(ConversionError::InvalidLeapSecond);
        }
    }
    let fraction = Nanosecond::new(nanos as f64).to::<qtty::unit::Second>();
    let utc = SecondsSplit::new(unix_whole + unix_origin(), fraction);
    let tai = utc.add_quantity(utc_offset_at_seconds(utc, segment));
    // A negative historical step gives two civil labels for the same TAI
    // instant. Keep the later segment authoritative in both directions;
    // reject labels in the superseded tail instead of silently changing them.
    if nanos < 1_000_000_000
        && segments.get(idx).is_some_and(|next| {
            tai.difference(tai_at_midnight(Day::new(next.start_mjd as f64), *next))
                >= Second::new(0.0)
        })
    {
        return Err(ConversionError::UtcHistoryUnsupported);
    }
    Ok(tai.pair())
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

fn utc_offset_at_seconds(utc: SecondsSplit, segment: UtcTaiSegment) -> Second {
    let reference = day_to_j2000_seconds::<MJD>(Day::new(segment.reference_mjd));
    let days = utc.difference(SecondsSplit::from_quantity(reference))
        / Day::one().to::<qtty::unit::Second>();
    segment.base + Second::new(segment.slope_seconds_per_day) * days
}

fn tai_at_midnight(mjd: Day, segment: UtcTaiSegment) -> SecondsSplit {
    let utc = SecondsSplit::from_quantity(day_to_j2000_seconds::<MJD>(mjd));
    utc.add_quantity(utc_offset_at_seconds(utc, segment))
}

fn utc_seconds_in_segment(tai: SecondsSplit, segment: UtcTaiSegment) -> SecondsSplit {
    let reference = day_to_j2000_seconds::<MJD>(Day::new(segment.reference_mjd));
    let drift_rate = Second::new(segment.slope_seconds_per_day)
        .to::<qtty::unit::Day>()
        .value();
    let uncorrected = tai.sub_quantity(segment.base);
    // t = u + base + k (u - reference), hence
    // u = (t - base) - k/(1+k) * (t - base - reference).
    // Only the small drift correction is multiplied, not the large coordinate.
    // Collapsing the displacement here costs < 1e-6 ns after multiplication
    // by |k| <= 3e-8 over the defined historical segments.
    let correction = uncorrected.difference(SecondsSplit::from_quantity(reference))
        * (drift_rate / (1.0 + drift_rate));
    uncorrected.sub_quantity(correction)
}

// Combine the large origins with the high word first. Adding an epoch-sized
// displacement to the low word would erase subsecond compensation.
fn shift(value: SecondsSplit, displacement: Second) -> SecondsSplit {
    let shifted = SecondsSplit::new(value.hi(), displacement);
    SecondsSplit::new(shifted.hi(), shifted.lo() + value.lo())
}

// UTC switched to integral offsets and SI seconds on 1972-01-01 (MJD
// 41317). Only subsequent +1 s steps between zero-drift segments are
// positive leap seconds in the context's UTC history (IERS UTC-TAI.history).
fn is_positive_leap(segment: UtcTaiSegment, next: UtcTaiSegment) -> bool {
    next.start_mjd > 41_317
        && segment.end_mjd == Some(next.start_mjd)
        && segment.slope_seconds_per_day == 0.0
        && next.slope_seconds_per_day == 0.0
        && segment.base == segment.base.floor()
        && next.base - segment.base == Second::one()
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
            if !is_positive_leap(segment, next) {
                // Historical positive steps leave TAI intervals with no civil
                // UTC label. Do not extend the preceding segment into the gap.
                return Err(ConversionError::UtcHistoryUnsupported);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::runtime_data::compiled_time_data;

    #[test]
    fn negative_step_cannot_preserve_both_historical_labels() {
        let data = compiled_time_data();
        let old = *data
            .utc_tai_segments()
            .iter()
            .find(|s| s.end_mjd == Some(39_887))
            .unwrap();
        let earlier = "1968-01-31T23:59:59.950000000Z"
            .parse::<DateTime<Utc>>()
            .unwrap();
        // Construct the earlier branch explicitly, bypassing the ambiguity
        // rejection. Its canonical inverse is on the later branch.
        let utc = SecondsSplit::new(
            Second::new(earlier.timestamp() as f64) + unix_origin(),
            Second::new(0.95),
        );
        let tai = utc.add_quantity(utc_offset_at_seconds(utc, old));
        let canonical = time_data_utc_from_tai_seconds(&data, tai.pair(), false).unwrap();
        assert_eq!(
            canonical,
            "1968-02-01T00:00:00.049999997Z"
                .parse::<DateTime<Utc>>()
                .unwrap()
        );
        assert_eq!(
            time_data_tai_seconds_from_utc(&data, earlier, false),
            Err(ConversionError::UtcHistoryUnsupported)
        );
    }

    #[test]
    fn every_segment_boundary_and_adjacent_nanoseconds() {
        let data = compiled_time_data();
        let segments = data.utc_tai_segments();
        for (idx, segment) in segments.iter().copied().enumerate() {
            let unix = mjd_to_unix_seconds(Day::new(segment.start_mjd as f64)).value() as i64;
            for delta in [
                -1_000_000_001_i64,
                -1_000_000_000,
                -999_999_999,
                -1,
                0,
                1,
                999_999_999,
                1_000_000_000,
            ] {
                let dt = DateTime::<Utc>::from_timestamp(unix, 0).unwrap()
                    + chrono::TimeDelta::nanoseconds(delta);
                let result = time_data_tai_seconds_from_utc(&data, dt, false);
                if idx == 0 && delta < 0 {
                    assert_eq!(result, Err(ConversionError::UtcBeforeDefinition));
                    continue;
                }
                let negative_step = idx > 0
                    && tai_at_midnight(Day::new(segment.start_mjd as f64), segments[idx - 1])
                        .difference(tai_at_midnight(Day::new(segment.start_mjd as f64), segment))
                        > Second::new(0.01);
                if delta == -1 && negative_step {
                    assert_eq!(result, Err(ConversionError::UtcHistoryUnsupported), "{dt}");
                    continue;
                }
                let pair = result.unwrap_or_else(|e| panic!("{dt}: {e:?}"));
                assert_eq!(
                    time_data_utc_from_tai_seconds(&data, pair, false).unwrap(),
                    dt,
                    "{dt}"
                );
                assert!(!time_data_tai_seconds_is_in_leap_window(&data, pair));
            }
            if idx == 0 {
                continue;
            }
            let previous = segments[idx - 1];
            let start = tai_at_midnight(Day::new(segment.start_mjd as f64), previous);
            let end = tai_at_midnight(Day::new(segment.start_mjd as f64), segment);
            if end.difference(start) > Second::new(0.01) {
                let middle = start.add_quantity(end.difference(start) / 2.0);
                if is_positive_leap(previous, segment) {
                    for nanos in [0, 1, 123_456_789, 999_999_999] {
                        let dt = DateTime::<Utc>::from_timestamp(unix - 1, 1_000_000_000 + nanos)
                            .unwrap();
                        let pair = time_data_tai_seconds_from_utc(&data, dt, false).unwrap();
                        assert!(time_data_tai_seconds_is_in_leap_window(&data, pair));
                        assert_eq!(
                            time_data_utc_from_tai_seconds(&data, pair, false).unwrap(),
                            dt
                        );
                    }
                } else {
                    assert!(!time_data_tai_seconds_is_in_leap_window(
                        &data,
                        middle.pair()
                    ));
                    assert_eq!(
                        time_data_utc_from_tai_seconds(&data, middle.pair(), false),
                        Err(ConversionError::UtcHistoryUnsupported)
                    );
                }
            }
        }
    }
}
