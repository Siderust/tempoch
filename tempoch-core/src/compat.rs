// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Backward-compatible re-exports and shims for items removed in 0.4.2.
//!
//! These items were part of the public API in tempoch < 0.4.2 and are retained
//! here to avoid forcing simultaneous upgrades of downstream crates.

use qtty::{unit, Day};

use crate::{
    constats,
    interval::Interval,
    representation::{JD, MJD},
    EncodedTime, Scale, TDB, TT,
};
use crate::representation::TimeRepresentation;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// J2000.0 epoch as `JulianDate<TT>` (JD 2 451 545.0 TT).
pub const J2000_TT: crate::JulianDate<TT> =
    EncodedTime::<TT, JD>::from_raw_unchecked(constats::J2000_JD_TT);

/// Days in a Julian year (365.25 d).
pub const JULIAN_YEAR_DAYS: Day = Day::new(365.25);

// ─────────────────────────────────────────────────────────────────────────────
// Free functions
// ─────────────────────────────────────────────────────────────────────────────

/// Return the complement of `periods` within `outer`.
///
/// Equivalent to `outer.complement(periods)`. Provided as a free function for
/// call-sites that imported it before the method API was introduced.
pub fn complement_within<T: Copy + PartialOrd>(
    outer: Interval<T>,
    periods: &[Interval<T>],
) -> Vec<Interval<T>> {
    outer.complement(periods)
}

// ─────────────────────────────────────────────────────────────────────────────
// TimeInstant trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait for instants that support day-offset arithmetic.
///
/// Root-finding routines use this to map a bracketed `[start, end]` interval
/// to `Day` offsets (`Quantity<unit::Day>`), solve, then map the root back to
/// `T`.
pub trait TimeInstant: Copy + PartialOrd {
    /// The duration quantity type: `qtty::Day = Quantity<unit::Day>`.
    type Duration: Copy;

    /// Days elapsed from `other` to `self`.
    fn difference(&self, other: &Self) -> Self::Duration;

    /// Return `self` shifted forward by `offset`.
    fn add_duration(&self, offset: Self::Duration) -> Self;

    /// Return the midpoint of `self` and `other`.
    fn mean(self, other: Self) -> Self;
}

impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> TimeInstant for EncodedTime<S, R> {
    /// `Day = Quantity<unit::Day>` — the day-offset quantity type.
    type Duration = Day;

    #[inline]
    fn difference(&self, other: &Self) -> Day {
        self.raw() - other.raw()
    }

    #[inline]
    fn add_duration(&self, offset: Day) -> Self {
        Self::from_raw_unchecked(self.raw() + offset)
    }

    #[inline]
    fn mean(self, other: Self) -> Self {
        Self::from_raw_unchecked((self.raw() + other.raw()) * 0.5)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Ord / Eq for day-based EncodedTime
//
// The pre-0.4.2 API exposed Ord on JulianDate and ModifiedJulianDate.
// NaN raw values are treated as equal-to and less-than any finite value to
// satisfy the `Ord` contract; well-formed code should never encounter NaN.
// ─────────────────────────────────────────────────────────────────────────────

impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> Eq for EncodedTime<S, R> {}

impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> Ord for EncodedTime<S, R> {
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.partial_cmp(other)
            .unwrap_or(core::cmp::Ordering::Equal)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Arithmetic operators for day-based EncodedTime
// ─────────────────────────────────────────────────────────────────────────────

impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> core::ops::Sub for EncodedTime<S, R> {
    type Output = Day;

    #[inline]
    fn sub(self, rhs: Self) -> Day {
        self.raw() - rhs.raw()
    }
}

impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> core::ops::Add<Day> for EncodedTime<S, R> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Day) -> Self {
        Self::from_raw_unchecked(self.raw() + rhs)
    }
}

impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> core::ops::Sub<Day> for EncodedTime<S, R> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Day) -> Self {
        Self::from_raw_unchecked(self.raw() - rhs)
    }
}

impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> core::ops::AddAssign<Day>
    for EncodedTime<S, R>
{
    #[inline]
    fn add_assign(&mut self, rhs: Day) {
        *self = Self::from_raw_unchecked(self.raw() + rhs);
    }
}

/// Add a raw day count (as `f64`) to this instant.
impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> core::ops::Add<f64> for EncodedTime<S, R> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: f64) -> Self {
        Self::from_raw_unchecked(self.raw() + Day::new(rhs))
    }
}

/// Subtract a raw day count (as `f64`) from this instant.
impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> core::ops::Sub<f64> for EncodedTime<S, R> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: f64) -> Self {
        Self::from_raw_unchecked(self.raw() - Day::new(rhs))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Representation conversions: MJD ↔ JD (same scale)
// ─────────────────────────────────────────────────────────────────────────────

impl<S: Scale> From<EncodedTime<S, MJD>> for EncodedTime<S, JD> {
    #[inline]
    fn from(mjd: EncodedTime<S, MJD>) -> Self {
        Self::from_raw_unchecked(mjd.raw() + constats::JD_MINUS_MJD)
    }
}

impl<S: Scale> From<EncodedTime<S, JD>> for EncodedTime<S, MJD> {
    #[inline]
    fn from(jd: EncodedTime<S, JD>) -> Self {
        Self::from_raw_unchecked(jd.raw() - constats::JD_MINUS_MJD)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience methods on any day-based EncodedTime
// ─────────────────────────────────────────────────────────────────────────────

impl<S: Scale, R: TimeRepresentation<Unit = unit::Day>> EncodedTime<S, R> {
    /// Construct from a raw day value without bounds checking.
    ///
    /// Deprecated: prefer [`EncodedTime::from_raw_unchecked`]`(Day::new(raw))`.
    #[inline]
    pub fn new(raw: f64) -> Self {
        Self::from_raw_unchecked(Day::new(raw))
    }

    /// Return the midpoint between `self` and `other` in representation space.
    ///
    /// Deprecated: prefer `(a.raw() + b.raw()) * 0.5`.
    #[inline]
    pub fn mean(self, other: Self) -> Self {
        Self::from_raw_unchecked((self.raw() + other.raw()) * 0.5)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience methods on JD-based EncodedTime
// ─────────────────────────────────────────────────────────────────────────────

impl<S: Scale> EncodedTime<S, JD> {
    /// Return the Julian Day number as a raw `f64`.
    ///
    /// Deprecated: prefer `self.raw().value()`.
    #[inline]
    pub fn jd_value(self) -> f64 {
        self.raw().value()
    }

    /// Julian centuries since J2000.0: `T = (JD − 2 451 545.0) / 36 525`.
    #[inline]
    pub fn julian_centuries(self) -> f64 {
        (self.raw().value() - constats::J2000_JD_TT.value()) / 36_525.0
    }

    /// Julian millennia since J2000.0: `T = (JD − 2 451 545.0) / 365 250`.
    #[inline]
    pub fn julian_millennias(self) -> f64 {
        (self.raw().value() - constats::J2000_JD_TT.value()) / 365_250.0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TT→TDB conversion on JD-encoded TT instants
// ─────────────────────────────────────────────────────────────────────────────

impl EncodedTime<TT, JD> {
    /// Convert a TT Julian Date to the equivalent TDB Julian Date.
    ///
    /// Deprecated: prefer `.to_time().to_scale::<TDB>().to::<JD>()`.
    #[inline]
    pub fn tt_to_tdb(jd: Self) -> EncodedTime<TDB, JD> {
        jd.to_time().to_scale::<TDB>().to::<JD>()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience methods on MJD-based EncodedTime
// ─────────────────────────────────────────────────────────────────────────────

impl<S: Scale> EncodedTime<S, MJD> {
    /// Return the Modified Julian Day number as a raw `f64`.
    ///
    /// Deprecated: prefer `self.raw().value()`.
    #[inline]
    pub fn mjd_value(self) -> f64 {
        self.raw().value()
    }
}
