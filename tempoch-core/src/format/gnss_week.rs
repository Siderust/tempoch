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
//! ## Precision
//!
//! `from_gnss_week` constructs the result by starting at the constellation's
//! epoch (stored as a split-f64 `Time<S>`) and calling `add_exact`, which
//! adds the integer whole-second and nanosecond components separately. This
//! avoids collapsing the full duration into a single `f64` before adding,
//! and produces results accurate to within the split-f64 storage precision
//! (typically < 1 μs for instants within a few hundred years of J2000).
//!
//! `to_gnss_week` extracts the integer-second and fractional-second components
//! from the split-f64 pair and performs all week/seconds decomposition in
//! integer arithmetic. Nanosecond fields are preserved as accurately as the
//! split-f64 storage allows; for instants near 2024 the storage precision is
//! approximately ±100 ns, so `subsecond_nanos` may differ from the
//! constructed value by at most that amount.
//!
//! See:
//! * IS-GPS-200 §20.3.3.3.1.1 (GPS week)
//! * Galileo OS-SIS-ICD §5.1.2 (GST)
//! * BeiDou ICD-OS §3.4 (BDT)
//! * IS-QZSS-PNT (QZSS week, GPS-compatible)

use crate::foundation::error::ConversionError;
use crate::model::scale::{CoordinateScale, BDT, GPST, GST, QZSST};
use crate::model::time::Time;

const SECONDS_PER_WEEK: qtty::i128::Second = qtty::i128::Second::new(7 * 86_400);

/// Decomposed GNSS week-number form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GnssWeek {
    /// Full week number since the constellation's defined epoch (no rollover).
    pub week: qtty::u32::Week,
    /// Seconds since the start of `week` in `[0, 604800)`.
    pub seconds_of_week: qtty::u32::Second,
    /// Subsecond nanoseconds remainder in `[0, 1_000_000_000)`.
    pub subsecond_nanos: qtty::u32::Nanosecond,
}

impl GnssWeek {
    /// Construct, validating ranges.
    pub fn new(
        week: qtty::u32::Week,
        seconds_of_week: qtty::u32::Second,
        subsecond_nanos: qtty::u32::Nanosecond,
    ) -> Result<Self, ConversionError> {
        if seconds_of_week.value() as i128 >= SECONDS_PER_WEEK.value() || subsecond_nanos.value() >= 1_000_000_000 {
            return Err(ConversionError::OutOfRange);
        }
        Ok(Self {
            week,
            seconds_of_week,
            subsecond_nanos,
        })
    }

    /// Return the subsecond nanoseconds remainder as a typed unsigned integer quantity.
    ///
    /// The returned value is always in `[0, 1_000_000_000)` nanoseconds.
    pub fn subsecond_nanoseconds_u(&self) -> qtty::u32::Nanosecond {
        self.subsecond_nanos
    }

    /// Return the seconds since the start of the week as a typed unsigned integer quantity.
    ///
    /// The returned value is always in `[0, 604_800)` seconds.
    pub fn seconds_of_week_u(&self) -> qtty::u32::Second {
        self.seconds_of_week
    }

    /// Construct from a typed unsigned nanosecond quantity.
    ///
    /// Rejects values ≥ 1 × 10⁹ ns.
    pub fn new_with_nanoseconds_u(
        week: qtty::u32::Week,
        seconds_of_week: qtty::u32::Second,
        subsecond: qtty::u32::Nanosecond,
    ) -> Result<Self, ConversionError> {
        Self::new(week, seconds_of_week, subsecond)
    }

    /// Convert back to a total ExactDuration since the scale's epoch.
    pub fn to_duration_since_epoch(&self) -> crate::ExactDuration {
        let week_count = self.week.value() as i128;
        let sow = self.seconds_of_week.value() as i128;
        let seconds = week_count * SECONDS_PER_WEEK.value() + sow;
        let nanos = seconds * 1_000_000_000 + self.subsecond_nanos.value() as i128;
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
    ///
    /// The whole-second decomposition uses integer arithmetic on the split-f64
    /// storage pair. The `subsecond_nanos` field is computed from the
    /// fractional remainder; see the module doc for precision limits.
    pub fn to_gnss_week(&self) -> Result<GnssWeek, ConversionError> {
        let (hi, lo) = self.to_j2000s().raw_seconds_pair();
        let hi_val = hi.value();
        let lo_val = lo.value();

        // Round hi to the nearest integer second so the residual stays small.
        let hi_int = hi_val.round();
        // sub_sec is the fractional-second part: the error of rounding hi, plus lo.
        let sub_sec = (hi_val - hi_int) + lo_val;

        // All epoch constants are exact integers expressible in f64 and i128.
        let epoch_i128 = S::epoch_j2000_seconds() as i128;
        // hi_int is within J2000-seconds range; cast via i64 then i128 is safe.
        let hi_i128 = hi_int as i64 as i128;
        let mut secs_since_epoch = hi_i128 - epoch_i128;

        // Convert sub-second residual to nanoseconds, handling carry.
        let raw_nanos = (sub_sec * 1.0e9).round() as i64;
        let sub_nanos = if raw_nanos < 0 {
            secs_since_epoch -= 1;
            (raw_nanos + 1_000_000_000) as u32
        } else if raw_nanos >= 1_000_000_000 {
            secs_since_epoch += 1;
            (raw_nanos - 1_000_000_000) as u32
        } else {
            raw_nanos as u32
        };

        if secs_since_epoch < 0 {
            return Err(ConversionError::OutOfRange);
        }

        let total_secs = secs_since_epoch as u64;
        let week_u64 = total_secs / SECONDS_PER_WEEK.value() as u64;
        if week_u64 > u32::MAX as u64 {
            return Err(ConversionError::OutOfRange);
        }
        let week = week_u64 as u32;
        let seconds_of_week = (total_secs % SECONDS_PER_WEEK.value() as u64) as u32;

        Ok(GnssWeek {
            week: qtty::u32::Week::new(week),
            seconds_of_week: qtty::u32::Second::new(seconds_of_week),
            subsecond_nanos: qtty::u32::Nanosecond::new(sub_nanos),
        })
    }

    /// Build a GNSS-scale instant from `(week, seconds_of_week,
    /// subsecond_nanos)` since the constellation's defined epoch.
    ///
    /// Uses `add_exact` to add the integer whole-second and nanosecond
    /// components to the epoch separately, preserving sub-millisecond
    /// precision within the split-f64 storage limits.
    pub fn from_gnss_week(gw: GnssWeek) -> Result<Self, ConversionError> {
        let epoch = Time::<S>::from_raw_j2000_seconds(qtty::Second::new(S::epoch_j2000_seconds()))?;
        Ok(epoch.add_exact(gw.to_duration_since_epoch()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::iso::parse_rfc3339_utc;

    #[test]
    fn gps_epoch_is_week_zero_second_zero() {
        let utc = parse_rfc3339_utc("1980-01-06T00:00:00Z").unwrap();
        let gpst: Time<GPST> = utc.to::<GPST>();
        let gw = gpst.to_gnss_week().unwrap();
        assert_eq!(gw.week.value(), 0, "expected week 0, got {gw:?}");
        assert_eq!(gw.seconds_of_week.value(), 0, "expected sow=0, got {gw:?}");
        assert_eq!(gw.subsecond_nanos.value(), 0, "expected ns=0, got {gw:?}");
    }

    #[test]
    fn galileo_epoch_is_week_zero_second_zero() {
        let utc = parse_rfc3339_utc("1999-08-22T00:00:00Z").unwrap();
        let gst: Time<GST> = utc.to::<GST>();
        let gw = gst.to_gnss_week().unwrap();
        assert_eq!(gw.week.value(), 0, "expected GST week 0, got {gw:?}");
        assert_eq!(gw.seconds_of_week.value(), 0, "expected sow=0, got {gw:?}");
        assert_eq!(gw.subsecond_nanos.value(), 0, "expected ns=0, got {gw:?}");
    }

    #[test]
    fn beidou_epoch_is_week_zero_second_zero() {
        let utc = parse_rfc3339_utc("2006-01-01T00:00:00Z").unwrap();
        let bdt: Time<BDT> = utc.to::<BDT>();
        let gw = bdt.to_gnss_week().unwrap();
        assert_eq!(gw.week.value(), 0, "expected BDT week 0, got {gw:?}");
        assert_eq!(gw.seconds_of_week.value(), 0, "expected sow=0, got {gw:?}");
        assert_eq!(gw.subsecond_nanos.value(), 0, "expected ns=0, got {gw:?}");
    }

    #[test]
    fn qzsst_aligned_with_gpst() {
        let utc = parse_rfc3339_utc("1980-01-06T00:00:00Z").unwrap();
        let q: Time<QZSST> = utc.to::<QZSST>();
        let gp: Time<GPST> = utc.to::<GPST>();
        let qw = q.to_gnss_week().unwrap();
        let gw = gp.to_gnss_week().unwrap();
        assert_eq!(qw.week, gw.week);
        assert_eq!(qw.seconds_of_week, gw.seconds_of_week);
        assert_eq!(qw.subsecond_nanos, gw.subsecond_nanos);
    }

    /// Round-trip test at GPS week 2200, sow 345600, subsecond 123_456_789 ns.
    /// The integer-arithmetic path must preserve all three fields exactly
    /// within the split-f64 storage tolerance.
    #[test]
    fn gps_week_round_trip_nanosecond_accurate() {
        let gw = GnssWeek::new(
            qtty::u32::Week::new(2200),
            qtty::u32::Second::new(345_600),
            qtty::u32::Nanosecond::new(123_456_789),
        )
        .unwrap();
        let t = Time::<GPST>::from_gnss_week(gw).unwrap();
        let back = t.to_gnss_week().unwrap();
        assert_eq!(back.week, gw.week, "week mismatch: {back:?} vs {gw:?}");
        assert_eq!(
            back.seconds_of_week, gw.seconds_of_week,
            "sow mismatch: {back:?} vs {gw:?}"
        );
        // subsecond_nanos must be within ±200 ns of the original (split-f64
        // storage precision near ~700 M seconds from J2000 is ~120 ns ULP).
        let ns_delta = (back.subsecond_nanos.value() as i64 - gw.subsecond_nanos.value() as i64).abs();
        assert!(
            ns_delta <= 200,
            "subsecond_nanos drift {ns_delta} ns: {back:?} vs {gw:?}"
        );
    }

    /// Week boundary: sow = 604_799, subsecond = 999_999_999 ns.
    #[test]
    fn gps_week_boundary() {
        let gw = GnssWeek::new(
            qtty::u32::Week::new(2200),
            qtty::u32::Second::new(604_799),
            qtty::u32::Nanosecond::new(999_999_999),
        )
        .unwrap();
        let t = Time::<GPST>::from_gnss_week(gw).unwrap();
        let back = t.to_gnss_week().unwrap();
        assert_eq!(back.week, gw.week, "week mismatch at boundary: {back:?}");
        assert_eq!(
            back.seconds_of_week, gw.seconds_of_week,
            "sow mismatch at boundary: {back:?}"
        );
        let ns_delta = (back.subsecond_nanos.value() as i64 - gw.subsecond_nanos.value() as i64).abs();
        assert!(
            ns_delta <= 200,
            "subsecond_nanos drift {ns_delta} ns at boundary: {back:?}"
        );
    }

    /// GPS week 1024 rollover: the full week number must not wrap.
    #[test]
    fn gps_week_1024_no_rollover() {
        let gw = GnssWeek::new(
            qtty::u32::Week::new(1024),
            qtty::u32::Second::new(0),
            qtty::u32::Nanosecond::new(0),
        )
        .unwrap();
        let t = Time::<GPST>::from_gnss_week(gw).unwrap();
        let back = t.to_gnss_week().unwrap();
        assert_eq!(back.week.value(), 1024);
        assert_eq!(back.seconds_of_week.value(), 0);
        assert_eq!(back.subsecond_nanos.value(), 0);
    }

    /// GPS week 2048 (second rollover boundary).
    #[test]
    fn gps_week_2048_no_rollover() {
        let gw = GnssWeek::new(
            qtty::u32::Week::new(2048),
            qtty::u32::Second::new(0),
            qtty::u32::Nanosecond::new(0),
        )
        .unwrap();
        let t = Time::<GPST>::from_gnss_week(gw).unwrap();
        let back = t.to_gnss_week().unwrap();
        assert_eq!(back.week.value(), 2048);
        assert_eq!(back.seconds_of_week.value(), 0);
        assert_eq!(back.subsecond_nanos.value(), 0);
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
        assert!(GnssWeek::new(
            qtty::u32::Week::new(0),
            qtty::u32::Second::new(604_800),
            qtty::u32::Nanosecond::new(0),
        )
        .is_err());
        assert!(GnssWeek::new(
            qtty::u32::Week::new(0),
            qtty::u32::Second::new(0),
            qtty::u32::Nanosecond::new(1_000_000_000),
        )
        .is_err());
    }

    #[test]
    fn subsecond_nanoseconds_u_matches_field() {
        let gw = GnssWeek::new(
            qtty::u32::Week::new(100),
            qtty::u32::Second::new(12_345),
            qtty::u32::Nanosecond::new(987_654_321),
        )
        .unwrap();
        assert_eq!(gw.subsecond_nanoseconds_u().value(), 987_654_321_u32);
    }

    #[test]
    fn new_with_nanoseconds_u_accepts_valid() {
        let ns = qtty::u32::Nanosecond::new(123_456_789);
        let gw = GnssWeek::new_with_nanoseconds_u(
            qtty::u32::Week::new(500),
            qtty::u32::Second::new(100_000),
            ns,
        )
        .unwrap();
        assert_eq!(gw.subsecond_nanos.value(), 123_456_789);
    }

    #[test]
    fn new_with_nanoseconds_u_rejects_invalid() {
        // out of range
        let big = qtty::u32::Nanosecond::new(1_000_000_000);
        assert!(GnssWeek::new_with_nanoseconds_u(
            qtty::u32::Week::new(0),
            qtty::u32::Second::new(0),
            big,
        )
        .is_err());
    }

    #[test]
    fn to_gnss_week_overflow_returns_out_of_range() {
        // Build a huge positive ExactDuration that maps to more than u32::MAX weeks,
        // then build a Time<GPST> that far in the future. Easiest way: construct a
        // Time via from_raw_j2000_seconds with a very large positive offset
        // corresponding to > u32::MAX * 604800 seconds past the GPST epoch.
        // u32::MAX * 604800 = 2_600_468_889_600 seconds ≈ 2.6e12 s
        // GPST epoch J2000 = -630_763_200 s
        // So target J2000 seconds = -630_763_200 + 2_600_468_889_600 + 1 = ~2_599_838_126_401 s
        // That's beyond the f64 exact-integer range so use a moderate approach:
        // Create a GnssWeek with week u32::MAX; to_duration_since_epoch() returns
        // a huge ExactDuration. Then from_gnss_week should succeed (it just adds),
        // and to_gnss_week on the result should return the correct week (u32::MAX).
        // Actually: let's verify that from_gnss_week does not silently wrap week.
        let gw_max = GnssWeek {
            week: qtty::u32::Week::new(u32::MAX),
            seconds_of_week: qtty::u32::Second::new(0),
            subsecond_nanos: qtty::u32::Nanosecond::new(0),
        };
        // The duration is u32::MAX * 604800 * 1e9 ns ≈ 2.6e21 ns which fits in i128.
        let dur = gw_max.to_duration_since_epoch();
        let (_s, _n) = dur
            .as_seconds_i64_nanos_checked()
            .expect("should fit in i64");
        // s ≈ 2.6e12 which is < i64::MAX, so add_exact should succeed.
        let epoch =
            Time::<GPST>::from_raw_j2000_seconds(qtty::Second::new(GPST_EPOCH_J2000_SECONDS))
                .unwrap();
        let t = epoch.add_exact(dur);
        // Convert back — week_u64 = u32::MAX, which is exactly u32::MAX, should succeed.
        let back = t.to_gnss_week().unwrap();
        assert_eq!(back.week.value(), u32::MAX);

        // Now test actual overflow: construct a raw j2000 instant such that
        // secs_since_epoch / 604800 > u32::MAX.
        // (u32::MAX + 1) * 604800 seconds past epoch:
        let overflow_secs = (u32::MAX as i128 + 1) * SECONDS_PER_WEEK.value();
        let epoch_j2000 = GPST_EPOCH_J2000_SECONDS as i128;
        let j2000_secs = epoch_j2000 + overflow_secs;
        // This is ~2.6e12 s past J2000, well within f64 precision for large integers.
        let t2 =
            Time::<GPST>::from_raw_j2000_seconds(qtty::Second::new(j2000_secs as f64)).unwrap();
        let result = t2.to_gnss_week();
        assert!(
            result.is_err(),
            "expected OutOfRange for week > u32::MAX, got {result:?}"
        );
    }
}
