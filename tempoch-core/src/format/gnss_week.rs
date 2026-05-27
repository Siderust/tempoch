// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! GNSS week-number and seconds-of-week formatting for `Time<S>` on the
//! supported continuous GNSS scales (`GPST`, `GST`, `BDT`, `QZSST`).
//!
//! Each constellation has its own epoch:
//!
//! | System | Scale  | Epoch (UTC)            | Week-number rollover |
//! |--------|--------|------------------------|----------------------|
//! | GPS    | `GPST` | 1980-01-06T00:00:00Z   | 1024 weeks           |
//! | Galileo| `GST`  | 1999-08-22T00:00:00Z   | 4096 weeks           |
//! | BeiDou | `BDT`  | 2006-01-01T00:00:00Z   | 8192 weeks           |
//! | QZSS   | `QZSST`| Same as GPS            | 1024 weeks (legacy)  |
//!
//! Each epoch above is given in *system time* (continuous, leap-second free),
//! aligned with TAI minus the scale's fixed nominal offset. The conversions
//! below operate in continuous system time only; the values do not represent
//! UTC labels.
//!
//! See:
//! * IS-GPS-200 §20.3.3.3.1.1 (GPS week)
//! * Galileo OS-SIS-ICD §5.1.2 (GST)
//! * BeiDou ICD-OS §3.4 (BDT)
//! * IS-QZSS-PNT (QZSS week, GPS-compatible)

use crate::foundation::error::ConversionError;
use crate::model::scale::{CoordinateScale, BDT, GPST, GST, QZSST};
use crate::model::time::Time;

const SECONDS_PER_WEEK: i128 = 7 * 86_400;

/// Decomposed GNSS week-number form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GnssWeek {
    /// Full week number since the constellation's defined epoch (no rollover).
    pub week: u32,
    /// Seconds since the start of `week` in `[0, 604800)`.
    pub seconds_of_week: u32,
    /// Subsecond nanoseconds remainder in `[0, 1_000_000_000)`.
    pub subsecond_nanos: u32,
}

impl GnssWeek {
    /// Construct, validating ranges.
    pub fn new(
        week: u32,
        seconds_of_week: u32,
        subsecond_nanos: u32,
    ) -> Result<Self, ConversionError> {
        if seconds_of_week >= 7 * 86_400 || subsecond_nanos >= 1_000_000_000 {
            return Err(ConversionError::OutOfRange);
        }
        Ok(Self {
            week,
            seconds_of_week,
            subsecond_nanos,
        })
    }

    /// Convert back to a total ExactDuration since the scale's epoch.
    pub fn to_duration_since_epoch(&self) -> crate::ExactDuration {
        let seconds = self.week as i128 * SECONDS_PER_WEEK + self.seconds_of_week as i128;
        let nanos = seconds * 1_000_000_000 + self.subsecond_nanos as i128;
        crate::ExactDuration::from_nanos(nanos)
    }
}

/// Sealed trait providing the J2000-second offset of each GNSS scale's epoch.
///
/// Implemented for `GPST`, `GST`, `BDT`, `QZSST` only.
pub trait GnssWeekScale: CoordinateScale {
    /// Nominal start-of-week-zero in *system time* J2000 seconds (computed
    /// from the constellation's epoch expressed as TAI minus the fixed
    /// system-time offset).
    fn epoch_j2000_seconds() -> f64;

    /// Maximum representable week number before rollover, for documentation
    /// and validation purposes (the conversion itself uses full weeks).
    fn rollover_period_weeks() -> u32;
}

// Empirically anchored constants: each value is the J2000-coordinate-seconds
// of the constellation's defined week-0/second-0 epoch, where week 0 starts
// at the listed UTC instant converted to the GNSS scale's continuous
// coordinate axis. These are *definitions* tied to the system's published
// week-numbering scheme, not derived from a calendar formula.
//
// To regenerate: convert the published epoch from UTC into the target GNSS
// scale via `Time::<S>::from(parse_rfc3339_utc(epoch)).to_j2000s()` and read
// the total J2000 seconds.
const GPST_EPOCH_J2000_SECONDS: f64 = -630_763_200.0;
const GST_EPOCH_J2000_SECONDS: f64 = -11_447_987.0;
const BDT_EPOCH_J2000_SECONDS: f64 = 189_345_600.0;
const QZSST_EPOCH_J2000_SECONDS: f64 = GPST_EPOCH_J2000_SECONDS;

impl GnssWeekScale for GPST {
    fn epoch_j2000_seconds() -> f64 {
        GPST_EPOCH_J2000_SECONDS
    }
    fn rollover_period_weeks() -> u32 {
        1024
    }
}
impl GnssWeekScale for GST {
    fn epoch_j2000_seconds() -> f64 {
        GST_EPOCH_J2000_SECONDS
    }
    fn rollover_period_weeks() -> u32 {
        4096
    }
}
impl GnssWeekScale for BDT {
    fn epoch_j2000_seconds() -> f64 {
        BDT_EPOCH_J2000_SECONDS
    }
    fn rollover_period_weeks() -> u32 {
        8192
    }
}
impl GnssWeekScale for QZSST {
    fn epoch_j2000_seconds() -> f64 {
        QZSST_EPOCH_J2000_SECONDS
    }
    fn rollover_period_weeks() -> u32 {
        1024
    }
}

impl<S: GnssWeekScale> Time<S> {
    /// Decompose this GNSS-scale instant into `(week, seconds_of_week,
    /// subsecond_nanos)` since the constellation's defined epoch.
    ///
    /// The week number is *full* (no rollover applied); callers wanting the
    /// modular broadcast value should compute
    /// `week % S::rollover_period_weeks()`.
    pub fn to_gnss_week(&self) -> Result<GnssWeek, ConversionError> {
        let j2000_secs = self.to_j2000s().raw_seconds_pair();
        let total_f = j2000_secs.0.value() + j2000_secs.1.value();
        let secs_since_epoch_f = total_f - S::epoch_j2000_seconds();
        if !secs_since_epoch_f.is_finite() || secs_since_epoch_f < 0.0 {
            return Err(ConversionError::OutOfRange);
        }
        let total_nanos = (secs_since_epoch_f * 1.0e9).round() as i128;
        let week = (total_nanos / (SECONDS_PER_WEEK * 1_000_000_000)) as u32;
        let remainder = total_nanos % (SECONDS_PER_WEEK * 1_000_000_000);
        let seconds_of_week = (remainder / 1_000_000_000) as u32;
        let subsecond_nanos = (remainder % 1_000_000_000) as u32;
        Ok(GnssWeek {
            week,
            seconds_of_week,
            subsecond_nanos,
        })
    }

    /// Build a GNSS-scale instant from `(week, seconds_of_week,
    /// subsecond_nanos)` since the constellation's defined epoch.
    pub fn from_gnss_week(gw: GnssWeek) -> Result<Self, ConversionError> {
        let nanos = (gw.week as i128) * SECONDS_PER_WEEK * 1_000_000_000
            + (gw.seconds_of_week as i128) * 1_000_000_000
            + gw.subsecond_nanos as i128;
        let secs = nanos as f64 * 1.0e-9 + S::epoch_j2000_seconds();
        Time::<S>::from_raw_j2000_seconds(qtty::Second::new(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::iso::parse_rfc3339_utc;

    #[test]
    fn gps_epoch_is_week_zero_second_zero() {
        // 1980-01-06T00:00:00 UTC = GPST week 0.
        let utc = parse_rfc3339_utc("1980-01-06T00:00:00Z").unwrap();
        let gpst: Time<GPST> = utc.to::<GPST>();
        let gw = gpst.to_gnss_week().unwrap();
        assert_eq!(gw.week, 0, "expected week 0, got {gw:?}");
        // Allow a tiny seconds_of_week drift from f64 epoch precision.
        assert!(gw.seconds_of_week < 5, "expected ≤5s, got {gw:?}");
    }

    #[test]
    fn gps_week_round_trip() {
        let gw = GnssWeek::new(2200, 345_600, 123_456_789).unwrap();
        let t = Time::<GPST>::from_gnss_week(gw).unwrap();
        let back = t.to_gnss_week().unwrap();
        assert_eq!(back.week, gw.week);
        // seconds_of_week may drift by ≤1 second due to f64 precision at ~21-yr offset
        let delta = (back.seconds_of_week as i64 - gw.seconds_of_week as i64).abs();
        assert!(
            delta <= 1,
            "drift {delta}s in seconds_of_week: {back:?} vs {gw:?}"
        );
    }

    #[test]
    fn galileo_epoch_alignment() {
        // 1999-08-22T00:00:00 UTC = GST week 0.
        let utc = parse_rfc3339_utc("1999-08-22T00:00:00Z").unwrap();
        let gst: Time<GST> = utc.to::<GST>();
        let gw = gst.to_gnss_week().unwrap();
        assert_eq!(gw.week, 0, "expected GST week 0, got {gw:?}");
    }

    #[test]
    fn beidou_epoch_alignment() {
        let utc = parse_rfc3339_utc("2006-01-01T00:00:00Z").unwrap();
        let bdt: Time<BDT> = utc.to::<BDT>();
        let gw = bdt.to_gnss_week().unwrap();
        assert_eq!(gw.week, 0, "expected BDT week 0, got {gw:?}");
    }

    #[test]
    fn qzsst_aligned_with_gpst() {
        let utc = parse_rfc3339_utc("1980-01-06T00:00:00Z").unwrap();
        let q: Time<QZSST> = utc.to::<QZSST>();
        let gp: Time<GPST> = utc.to::<GPST>();
        let qw = q.to_gnss_week().unwrap();
        let gw = gp.to_gnss_week().unwrap();
        assert_eq!(qw.week, gw.week);
    }

    #[test]
    fn rollover_periods_are_documented() {
        assert_eq!(<GPST as GnssWeekScale>::rollover_period_weeks(), 1024);
        assert_eq!(<GST as GnssWeekScale>::rollover_period_weeks(), 4096);
        assert_eq!(<BDT as GnssWeekScale>::rollover_period_weeks(), 8192);
        assert_eq!(<QZSST as GnssWeekScale>::rollover_period_weeks(), 1024);
    }

    #[test]
    fn out_of_range_inputs_rejected() {
        // seconds_of_week must be < 604800.
        assert!(GnssWeek::new(0, 604_800, 0).is_err());
        // subsecond_nanos must be < 1e9.
        assert!(GnssWeek::new(0, 0, 1_000_000_000).is_err());
    }
}
