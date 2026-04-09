// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Generic time–scale parameterised instant.
//!
//! [`Time<S>`] is the core type of the time module.  It stores a scalar
//! quantity in [`Day`] whose *meaning* is determined by the compile-time
//! marker `S: TimeScale`.  All arithmetic (addition/subtraction of
//! durations, difference between instants), UTC conversion, serialisation,
//! and display are implemented generically — no code duplication.
//!
//! Domain-specific methods that only make sense for a particular scale
//! (e.g. [`Time::<JD>::julian_centuries()`]) are placed in inherent `impl`
//! blocks gated on the concrete marker type.

use chrono::{DateTime, Utc};
use qtty::*;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// ═══════════════════════════════════════════════════════════════════════════
// TimeScale trait
// ═══════════════════════════════════════════════════════════════════════════

/// Marker trait for time scales.
///
/// A **time scale** defines:
///
/// 1. A human-readable **label** (e.g. `"JD"`, `"MJD"`, `"TAI"`).
/// 2. A pair of conversion functions between the scale's native quantity
///    (in [`Day`]) and **Julian Date in TT** (JD(TT)) — the canonical
///    internal representation used throughout the crate.
///
/// For pure *epoch counters* (JD, MJD, Unix Time, GPS) the conversions are
/// trivial constant offsets that the compiler will inline and fold away.
///
/// For *physical scales* (TT, TDB, TAI) the conversions may include
/// function-based corrections (e.g. the ≈1.7 ms TDB↔TT periodic term).
pub trait TimeScale: Copy + Clone + std::fmt::Debug + PartialEq + PartialOrd + 'static {
    /// Display label used by [`Time`] formatting.
    const LABEL: &'static str;

    /// Convert a quantity in this scale's native unit to an absolute JD(TT).
    fn to_jd_tt(value: Day) -> Day;

    /// Convert an absolute JD(TT) back to this scale's native quantity.
    fn from_jd_tt(jd_tt: Day) -> Day;
}

// ═══════════════════════════════════════════════════════════════════════════
// Error types
// ═══════════════════════════════════════════════════════════════════════════

/// Error returned when a `Time` value is non-finite (`NaN` or `±∞`).
///
/// Non-finite values break ordering, intersection, and arithmetic invariants,
/// so validated constructors ([`Time::try_new`], [`Time::try_from_days`])
/// reject them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonFiniteTimeError;

impl std::fmt::Display for NonFiniteTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "time value must be finite (not NaN or infinity)")
    }
}

impl std::error::Error for NonFiniteTimeError {}

// ═══════════════════════════════════════════════════════════════════════════
// Time<S> — the generic instant
// ═══════════════════════════════════════════════════════════════════════════

/// A point on time scale `S`.
///
/// Internally stores a single `Day` quantity whose interpretation depends on
/// `S: TimeScale`.  The struct is `Copy` and zero-cost: `PhantomData` is
/// zero-sized, so `Time<S>` is layout-identical to `Day` (a single `f64`).
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Time<S: TimeScale> {
    quantity: Day,
    _scale: PhantomData<S>,
}

impl<S: TimeScale> Time<S> {
    // ── constructors ──────────────────────────────────────────────────

    /// Create from a raw scalar (days since the scale's epoch).
    ///
    /// **Note:** this constructor accepts any `f64`, including `NaN` and `±∞`.
    /// Prefer [`try_new`](Self::try_new) when the value comes from untrusted
    /// or computed input.
    #[inline]
    pub const fn new(value: f64) -> Self {
        Self {
            quantity: Day::new(value),
            _scale: PhantomData,
        }
    }

    /// Create from a raw scalar, rejecting non-finite values.
    ///
    /// Returns [`NonFiniteTimeError`] if `value` is `NaN`, `+∞`, or `−∞`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use tempoch_core as tempoch;
    /// use tempoch::{Time, JD};
    ///
    /// assert!(Time::<JD>::try_new(2451545.0).is_ok());
    /// assert!(Time::<JD>::try_new(f64::NAN).is_err());
    /// assert!(Time::<JD>::try_new(f64::INFINITY).is_err());
    /// ```
    #[inline]
    pub fn try_new(value: f64) -> Result<Self, NonFiniteTimeError> {
        if value.is_finite() {
            Ok(Self::new(value))
        } else {
            Err(NonFiniteTimeError)
        }
    }

    /// Create from a [`Day`] quantity.
    ///
    /// **Note:** this constructor accepts any `f64`, including `NaN` and `±∞`.
    /// Prefer [`try_from_days`](Self::try_from_days) when the value comes from
    /// untrusted or computed input.
    #[inline]
    pub const fn from_days(days: Day) -> Self {
        Self {
            quantity: days,
            _scale: PhantomData,
        }
    }

    /// Create from a [`Day`] quantity, rejecting non-finite values.
    ///
    /// Returns [`NonFiniteTimeError`] if the underlying value is `NaN`,
    /// `+∞`, or `−∞`.
    #[inline]
    pub fn try_from_days(days: Day) -> Result<Self, NonFiniteTimeError> {
        Self::try_new(days.value())
    }

    // ── accessors ─────────────────────────────────────────────────────

    /// The underlying quantity in days.
    #[inline]
    pub const fn quantity(&self) -> Day {
        self.quantity
    }

    /// The underlying scalar value in days.
    #[inline]
    pub const fn value(&self) -> f64 {
        self.quantity.value()
    }

    /// Absolute Julian Day (TT) corresponding to this instant.
    #[inline]
    pub fn julian_day(&self) -> Day {
        S::to_jd_tt(self.quantity)
    }

    /// Absolute Julian Day (TT) as scalar.
    #[inline]
    pub fn julian_day_value(&self) -> f64 {
        self.julian_day().value()
    }

    /// Build an instant from an absolute Julian Day (TT).
    #[inline]
    pub fn from_julian_day(jd: Day) -> Self {
        Self::from_days(S::from_jd_tt(jd))
    }

    // ── cross-scale conversion (mirroring qtty's .to::<T>()) ─────────

    /// Convert this instant to another time scale.
    ///
    /// The conversion routes through the canonical JD(TT) intermediate:
    ///
    /// ```text
    /// self → JD(TT) → target
    /// ```
    ///
    /// For pure epoch-offset scales this compiles down to a single
    /// addition/subtraction.
    #[inline]
    pub fn to<T: TimeScale>(&self) -> Time<T> {
        Time::<T>::from_julian_day(S::to_jd_tt(self.quantity))
    }

    // ── UTC helpers ───────────────────────────────────────────────────

    /// Convert to a `chrono::DateTime<Utc>`.
    ///
    /// Inverts the ΔT correction to recover the UTC / UT timestamp.
    /// Returns `None` if the value falls outside chrono's representable range.
    pub fn to_utc(&self) -> Option<DateTime<Utc>> {
        use super::scales::UT;
        const UNIX_EPOCH_JD: f64 = 2_440_587.5;
        let jd_ut = self.to::<UT>().quantity();
        let seconds_since_epoch = (jd_ut - Day::new(UNIX_EPOCH_JD))
            .to::<qtty::unit::Second>()
            .value();
        let secs = seconds_since_epoch.floor() as i64;
        let nanos = ((seconds_since_epoch - secs as f64) * 1e9) as u32;
        DateTime::<Utc>::from_timestamp(secs, nanos)
    }

    /// Build an instant from a `chrono::DateTime<Utc>`.
    ///
    /// The UTC timestamp is interpreted as Universal Time (≈ UT1) and the
    /// epoch-dependent **ΔT** correction is applied automatically, so the
    /// resulting `Time<S>` is on the target scale's axis.
    pub fn from_utc(datetime: DateTime<Utc>) -> Self {
        use super::scales::UT;
        const UNIX_EPOCH_JD: f64 = 2_440_587.5;
        let seconds_since_epoch = Second::new(datetime.timestamp() as f64);
        let nanos = Second::new(datetime.timestamp_subsec_nanos() as f64 / 1e9);
        let jd_ut = Day::new(UNIX_EPOCH_JD) + (seconds_since_epoch + nanos).to::<qtty::unit::Day>();
        Time::<UT>::from_days(jd_ut).to::<S>()
    }

    // ── min / max ─────────────────────────────────────────────────────

    /// Element-wise minimum.
    #[inline]
    pub const fn min(self, other: Self) -> Self {
        Self::from_days(self.quantity.min_const(other.quantity))
    }

    /// Element-wise maximum.
    #[inline]
    pub const fn max(self, other: Self) -> Self {
        Self::from_days(self.quantity.max_const(other.quantity))
    }

    /// Mean (midpoint) between two instants on the same time scale.
    #[inline]
    pub const fn mean(self, other: Self) -> Self {
        Self::from_days(self.quantity.const_add(other.quantity).const_div(2.0))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Generic trait implementations
// ═══════════════════════════════════════════════════════════════════════════

// ── Display ───────────────────────────────────────────────────────────────

impl<S: TimeScale> std::fmt::Display for Time<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Format: "JD 2451545.0" — scale label followed by the raw day value.
        // The `d` unit suffix is intentionally omitted: for time scales the
        // scale label already conveys the scale (JD, MJD, TT, …) and the
        // trailing `d` was redundant and visually confusing.
        // All format flags (precision, width, …) are forwarded to the f64
        // value so that e.g. `format!("{:.9}", my_jd)` works directly.
        write!(f, "{} ", S::LABEL)?;
        std::fmt::Display::fmt(&self.quantity.value(), f)
    }
}

// ── Serde ─────────────────────────────────────────────────────────────────

#[cfg(feature = "serde")]
impl<S: TimeScale> Serialize for Time<S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        serializer.serialize_f64(self.value())
    }
}

#[cfg(feature = "serde")]
impl<'de, S: TimeScale> Deserialize<'de> for Time<S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = f64::deserialize(deserializer)?;
        if !v.is_finite() {
            return Err(serde::de::Error::custom(
                "time value must be finite (not NaN or infinity)",
            ));
        }
        Ok(Self::new(v))
    }
}

// ── Arithmetic ────────────────────────────────────────────────────────────

impl<S: TimeScale> Add<Day> for Time<S> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Day) -> Self::Output {
        Self::from_days(self.quantity + rhs)
    }
}

impl<S: TimeScale> AddAssign<Day> for Time<S> {
    #[inline]
    fn add_assign(&mut self, rhs: Day) {
        self.quantity += rhs;
    }
}

impl<S: TimeScale> Sub<Day> for Time<S> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Day) -> Self::Output {
        Self::from_days(self.quantity - rhs)
    }
}

impl<S: TimeScale> SubAssign<Day> for Time<S> {
    #[inline]
    fn sub_assign(&mut self, rhs: Day) {
        self.quantity -= rhs;
    }
}

impl<S: TimeScale> Sub for Time<S> {
    type Output = Day;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self.quantity - rhs.quantity
    }
}

impl<S: TimeScale> std::ops::Div<Day> for Time<S> {
    type Output = f64;
    #[inline]
    fn div(self, rhs: Day) -> Self::Output {
        (self.quantity / rhs).value()
    }
}

impl<S: TimeScale> std::ops::Div<f64> for Time<S> {
    type Output = f64;
    #[inline]
    fn div(self, rhs: f64) -> Self::Output {
        (self.quantity / rhs).value()
    }
}

// ── From/Into Day ────────────────────────────────────────────────────────

impl<S: TimeScale> From<Day> for Time<S> {
    #[inline]
    fn from(days: Day) -> Self {
        Self::from_days(days)
    }
}

impl<S: TimeScale> From<Time<S>> for Day {
    #[inline]
    fn from(time: Time<S>) -> Self {
        time.quantity
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TimeInstant trait
// ═══════════════════════════════════════════════════════════════════════════

/// Trait for types that represent a point in time.
///
/// Types implementing this trait can be used as time instants in `Interval<T>`
/// and provide conversions to/from UTC plus basic arithmetic operations.
pub trait TimeInstant: Copy + Clone + PartialEq + PartialOrd + Sized {
    /// The duration type used for arithmetic operations.
    type Duration;

    /// Convert this time instant to UTC DateTime.
    fn to_utc(&self) -> Option<DateTime<Utc>>;

    /// Create a time instant from UTC DateTime.
    fn from_utc(datetime: DateTime<Utc>) -> Self;

    /// Compute the difference between two time instants.
    fn difference(&self, other: &Self) -> Self::Duration;

    /// Add a duration to this time instant.
    fn add_duration(&self, duration: Self::Duration) -> Self;

    /// Subtract a duration from this time instant.
    fn sub_duration(&self, duration: Self::Duration) -> Self;
}

impl<S: TimeScale> TimeInstant for Time<S> {
    type Duration = Day;

    #[inline]
    fn to_utc(&self) -> Option<DateTime<Utc>> {
        Time::to_utc(self)
    }

    #[inline]
    fn from_utc(datetime: DateTime<Utc>) -> Self {
        Time::from_utc(datetime)
    }

    #[inline]
    fn difference(&self, other: &Self) -> Self::Duration {
        *self - *other
    }

    #[inline]
    fn add_duration(&self, duration: Self::Duration) -> Self {
        *self + duration
    }

    #[inline]
    fn sub_duration(&self, duration: Self::Duration) -> Self {
        *self - duration
    }
}

impl TimeInstant for DateTime<Utc> {
    type Duration = chrono::Duration;

    fn to_utc(&self) -> Option<DateTime<Utc>> {
        Some(*self)
    }

    fn from_utc(datetime: DateTime<Utc>) -> Self {
        datetime
    }

    fn difference(&self, other: &Self) -> Self::Duration {
        *self - *other
    }

    fn add_duration(&self, duration: Self::Duration) -> Self {
        *self + duration
    }

    fn sub_duration(&self, duration: Self::Duration) -> Self {
        *self - duration
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::super::scales::{JD, MJD};
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_julian_day_creation() {
        let jd = Time::<JD>::new(2_451_545.0);
        assert_eq!(jd.quantity(), Day::new(2_451_545.0));
    }

    #[test]
    fn test_jd_utc_roundtrip() {
        // from_utc applies ΔT (UT→TT); to_utc inverts it (TT→UT).
        let datetime = DateTime::from_timestamp(946_728_000, 0).unwrap();
        let jd = Time::<JD>::from_utc(datetime);
        let back = jd.to_utc().expect("to_utc");
        let delta_ns =
            back.timestamp_nanos_opt().unwrap() - datetime.timestamp_nanos_opt().unwrap();
        assert!(delta_ns.abs() < 1_000, "roundtrip error: {} ns", delta_ns);
    }

    #[test]
    fn test_from_utc_applies_delta_t() {
        // 2000-01-01 12:00:00 UTC → JD(UT)=2451545.0; ΔT≈63.83 s
        let datetime = DateTime::from_timestamp(946_728_000, 0).unwrap();
        let jd = Time::<JD>::from_utc(datetime);
        let delta_t_secs = (jd.quantity() - Day::new(2_451_545.0)).to::<qtty::unit::Second>();
        assert!(
            (delta_t_secs - Second::new(63.83)).abs() < Second::new(1.0),
            "ΔT correction = {} s, expected ~63.83 s",
            delta_t_secs
        );
    }

    #[test]
    fn test_julian_conversions() {
        let jd = Time::<JD>::J2000 + Day::new(365_250.0);
        assert!((jd.julian_millennias() - Millennium::new(1.0)).abs() < Millennium::new(1e-12));
        assert!((jd.julian_centuries() - Century::new(10.0)).abs() < Century::new(1e-12));
        assert!((jd.julian_years() - JulianYear::new(1000.0)).abs() < JulianYear::new(1e-9));
    }

    #[test]
    fn test_tt_to_tdb_and_min_max() {
        let jd_tdb = Time::<JD>::tt_to_tdb(Time::<JD>::J2000);
        assert!((jd_tdb - Time::<JD>::J2000).abs() < Day::new(1e-6));

        let earlier = Time::<JD>::J2000;
        let later = earlier + Day::new(1.0);
        assert_eq!(earlier.min(later), earlier);
        assert_eq!(earlier.max(later), later);
    }

    #[test]
    fn test_const_min_max() {
        const A: Time<JD> = Time::<JD>::new(10.0);
        const B: Time<JD> = Time::<JD>::new(14.0);
        const MIN: Time<JD> = A.min(B);
        const MAX: Time<JD> = A.max(B);
        assert_eq!(MIN.quantity(), Day::new(10.0));
        assert_eq!(MAX.quantity(), Day::new(14.0));
    }

    #[test]
    fn test_mean_and_const_mean() {
        let a = Time::<JD>::new(10.0);
        let b = Time::<JD>::new(14.0);
        assert_eq!(a.mean(b).quantity(), Day::new(12.0));
        assert_eq!(b.mean(a).quantity(), Day::new(12.0));

        const MID: Time<JD> = Time::<JD>::new(10.0).mean(Time::<JD>::new(14.0));
        assert_eq!(MID.quantity(), Day::new(12.0));
    }

    #[test]
    fn test_into_days() {
        let jd = Time::<JD>::new(2_451_547.5);
        let days: Day = jd.into();
        assert_eq!(days, Day::new(2_451_547.5));

        let roundtrip = Time::<JD>::from(days);
        assert_eq!(roundtrip, jd);
    }

    #[test]
    fn test_into_julian_years() {
        let jd = Time::<JD>::J2000 + Day::new(365.25 * 2.0);
        let years: JulianYear = jd.into();
        assert!((years - JulianYear::new(2.0)).abs() < JulianYear::new(1e-12));

        let roundtrip = Time::<JD>::from(years);
        assert!((roundtrip.quantity() - jd.quantity()).abs() < Day::new(1e-12));
    }

    #[test]
    fn time_has_days_layout() {
        assert_eq!(std::mem::size_of::<Time<JD>>(), std::mem::size_of::<Day>());
        assert_eq!(
            std::mem::align_of::<Time<JD>>(),
            std::mem::align_of::<Day>()
        );
    }

    #[test]
    fn test_into_centuries() {
        let jd = Time::<JD>::J2000 + Day::new(36_525.0 * 3.0);
        let centuries: Century = jd.into();
        assert!((centuries - Century::new(3.0)).abs() < Century::new(1e-12));

        let roundtrip = Time::<JD>::from(centuries);
        assert!((roundtrip.quantity() - jd.quantity()).abs() < Day::new(1e-12));
    }

    #[test]
    fn test_into_millennia() {
        let jd = Time::<JD>::J2000 + Day::new(365_250.0 * 1.5);
        let millennia: Millennium = jd.into();
        assert!((millennia - Millennium::new(1.5)).abs() < Millennium::new(1e-12));

        let roundtrip = Time::<JD>::from(millennia);
        assert!((roundtrip.quantity() - jd.quantity()).abs() < Day::new(1e-9));
    }

    #[test]
    fn test_mjd_creation() {
        let mjd = Time::<MJD>::new(51_544.5);
        assert_eq!(mjd.quantity(), Day::new(51_544.5));
    }

    #[test]
    fn test_mjd_into_jd() {
        let mjd = Time::<MJD>::new(51_544.5);
        let jd: Time<JD> = mjd.into();
        assert_eq!(jd.quantity(), Day::new(2_451_545.0));
    }

    #[test]
    fn test_mjd_utc_roundtrip() {
        let datetime = DateTime::from_timestamp(946_728_000, 0).unwrap();
        let mjd = Time::<MJD>::from_utc(datetime);
        let back = mjd.to_utc().expect("to_utc");
        let delta_ns =
            back.timestamp_nanos_opt().unwrap() - datetime.timestamp_nanos_opt().unwrap();
        assert!(delta_ns.abs() < 1_000, "roundtrip error: {} ns", delta_ns);
    }

    #[test]
    fn test_mjd_from_utc_applies_delta_t() {
        // MJD epoch is JD − 2400000.5; ΔT should shift value by ~63.83/86400 days
        let datetime = DateTime::from_timestamp(946_728_000, 0).unwrap();
        let mjd = Time::<MJD>::from_utc(datetime);
        let delta_t_secs = (mjd.quantity() - Day::new(51_544.5)).to::<qtty::unit::Second>();
        assert!(
            (delta_t_secs - Second::new(63.83)).abs() < Second::new(1.0),
            "ΔT correction = {} s, expected ~63.83 s",
            delta_t_secs
        );
    }

    #[test]
    fn test_mjd_add_days() {
        let mjd = Time::<MJD>::new(59_000.0);
        let result = mjd + Day::new(1.5);
        assert_eq!(result.quantity(), Day::new(59_001.5));
    }

    #[test]
    fn test_mjd_sub_days() {
        let mjd = Time::<MJD>::new(59_000.0);
        let result = mjd - Day::new(1.5);
        assert_eq!(result.quantity(), Day::new(58_998.5));
    }

    #[test]
    fn test_mjd_sub_mjd() {
        let mjd1 = Time::<MJD>::new(59_001.0);
        let mjd2 = Time::<MJD>::new(59_000.0);
        let diff = mjd1 - mjd2;
        assert_eq!(diff, Day::new(1.0));
    }

    #[test]
    fn test_mjd_comparison() {
        let mjd1 = Time::<MJD>::new(59_000.0);
        let mjd2 = Time::<MJD>::new(59_001.0);
        assert!(mjd1 < mjd2);
        assert!(mjd2 > mjd1);
    }

    #[test]
    fn test_display_jd() {
        let jd = Time::<JD>::new(2_451_545.0);
        let s = format!("{jd}");
        assert!(s.contains("Julian Day"));
    }

    #[test]
    fn test_try_new_finite() {
        let jd = Time::<JD>::try_new(2_451_545.0);
        assert!(jd.is_ok());
        assert_eq!(jd.unwrap().value(), 2_451_545.0);
    }

    #[test]
    fn test_try_new_nan() {
        assert!(Time::<JD>::try_new(f64::NAN).is_err());
    }

    #[test]
    fn test_try_new_infinity() {
        assert!(Time::<JD>::try_new(f64::INFINITY).is_err());
        assert!(Time::<JD>::try_new(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn test_try_from_days() {
        assert!(Time::<JD>::try_from_days(Day::new(100.0)).is_ok());
        assert!(Time::<JD>::try_from_days(Day::new(f64::NAN)).is_err());
    }

    #[test]
    fn test_display_mjd() {
        let mjd = Time::<MJD>::new(51_544.5);
        let s = format!("{mjd}");
        assert!(s.contains("MJD"));
    }

    #[test]
    fn test_add_assign_sub_assign() {
        let mut jd = Time::<JD>::new(2_451_545.0);
        jd += Day::new(1.0);
        assert_eq!(jd.quantity(), Day::new(2_451_546.0));
        jd -= Day::new(0.5);
        assert_eq!(jd.quantity(), Day::new(2_451_545.5));
    }

    #[test]
    fn test_add_years() {
        let jd = Time::<JD>::new(2_450_000.0);
        let with_years = jd + Year::new(1.0);
        let span: Day = with_years - jd;
        assert!((span - Time::<JD>::JULIAN_YEAR).abs() < Day::new(1e-9));
    }

    #[test]
    fn test_div_days_and_f64() {
        let jd = Time::<JD>::new(100.0);
        assert!((jd / Day::new(2.0) - 50.0).abs() < 1e-12);
        assert!((jd / 4.0 - 25.0).abs() < 1e-12);
    }

    #[test]
    fn test_to_method_jd_mjd() {
        let jd = Time::<JD>::new(2_451_545.0);
        let mjd = jd.to::<MJD>();
        assert!((mjd.quantity() - Day::new(51_544.5)).abs() < Day::new(1e-10));
    }

    #[test]
    fn timeinstant_for_julian_date_handles_arithmetic() {
        let jd = Time::<JD>::new(2_451_545.0);
        let other = jd + Day::new(2.0);

        assert_eq!(jd.difference(&other), Day::new(-2.0));
        assert_eq!(
            jd.add_duration(Day::new(1.5)).quantity(),
            Day::new(2_451_546.5)
        );
        assert_eq!(
            other.sub_duration(Day::new(0.5)).quantity(),
            Day::new(2_451_546.5)
        );
    }

    #[test]
    fn timeinstant_for_modified_julian_date_roundtrips_utc() {
        let dt = DateTime::from_timestamp(946_684_800, 123_000_000).unwrap(); // 2000-01-01T00:00:00.123Z
        let mjd = Time::<MJD>::from_utc(dt);
        let back = mjd.to_utc().expect("mjd to utc");

        assert_eq!(mjd.difference(&mjd), Day::new(0.0));
        assert_eq!(
            mjd.add_duration(Day::new(1.0)).quantity(),
            mjd.quantity() + Day::new(1.0)
        );
        assert_eq!(
            mjd.sub_duration(Day::new(0.5)).quantity(),
            mjd.quantity() - Day::new(0.5)
        );
        let delta_ns = back.timestamp_nanos_opt().unwrap() - dt.timestamp_nanos_opt().unwrap();
        assert!(delta_ns.abs() < 10_000, "nanos differ by {}", delta_ns);
    }

    #[test]
    fn timeinstant_for_datetime_uses_chrono_durations() {
        let base = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2024, 1, 2, 6, 0, 0).unwrap();
        let diff = later.difference(&base);

        assert_eq!(diff.num_hours(), 30);
        assert_eq!(
            base.add_duration(diff + chrono::Duration::hours(6)),
            later + chrono::Duration::hours(6)
        );
        assert_eq!(later.sub_duration(diff), base);
        assert_eq!(TimeInstant::to_utc(&later), Some(later));
    }

    // ── New coverage tests ────────────────────────────────────────────

    #[test]
    fn test_non_finite_error_display() {
        let err = NonFiniteTimeError;
        let msg = format!("{err}");
        assert!(msg.contains("finite"), "unexpected: {msg}");
    }

    #[test]
    fn test_julian_day_and_julian_day_value() {
        // MJD 51544.5 == JD 2451545.0 (J2000.0 in TT).
        let mjd = Time::<MJD>::new(51_544.5);
        let jd_days = mjd.julian_day();
        assert!(
            (jd_days - Day::new(2_451_545.0)).abs() < Day::new(1e-10),
            "julian_day mismatch: {jd_days}"
        );
        assert!(
            (mjd.julian_day_value() - 2_451_545.0).abs() < 1e-10,
            "julian_day_value mismatch: {}",
            mjd.julian_day_value()
        );
    }

    #[test]
    fn test_timeinstant_trait_to_utc_and_from_utc_for_time() {
        // Call to_utc / from_utc through the TimeInstant trait (UFCS) so that
        // the forwarding wrapper functions in the TimeInstant impl are covered.
        let jd = Time::<JD>::new(2_451_545.0);
        let utc: Option<_> = TimeInstant::to_utc(&jd);
        assert!(utc.is_some());
        let back: Time<JD> = TimeInstant::from_utc(utc.unwrap());
        assert!((back.value() - jd.value()).abs() < 1e-6);
    }

    #[test]
    fn test_datetime_timeinstant_from_utc() {
        // Exercises TimeInstant::from_utc for DateTime<Utc>.
        let dt = DateTime::from_timestamp(0, 0).unwrap();
        let back: DateTime<Utc> = TimeInstant::from_utc(dt);
        assert_eq!(back, dt);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_serialize_time() {
        let jd = Time::<JD>::new(2_451_545.0);
        let json = serde_json::to_string(&jd).unwrap();
        assert!(json.contains("2451545"), "serialized: {json}");
        let back: Time<JD> = serde_json::from_str(&json).unwrap();
        assert_eq!(jd.value(), back.value());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_deserialize_nan_rejected() {
        use serde::{de::IntoDeserializer, Deserialize};
        let result: Result<Time<JD>, serde::de::value::Error> =
            Time::<JD>::deserialize(f64::NAN.into_deserializer());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("finite"), "unexpected error: {msg}");
    }
}
