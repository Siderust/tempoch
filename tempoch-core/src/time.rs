// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! `Time<A>` — the core public type.

use super::axis::Axis;
use super::context::TimeContext;
use super::conversion::{ContextConvertible, InfallibleConvertible};
use super::encoding::{
    j2000_seconds_to_jd, j2000_seconds_to_mjd, jd_to_j2000_seconds, mjd_to_j2000_seconds,
};
use super::error::ConversionError;
use super::storage::{ContinuousAxis, Storage};
use qtty::{Day, Second};

// ═══════════════════════════════════════════════════════════════════════════
// Time
// ═══════════════════════════════════════════════════════════════════════════

/// A point in time on axis `A`.
///
/// For continuous axes (`TT`, `TAI`, `TDB`, `TCG`, `TCB`, `UT1`) the
/// internal storage is SI seconds since J2000 TT counted on that axis.
///
/// `UTC` is an exception: storage holds the equivalent TAI seconds (keeping
/// the scalar continuous across leap seconds) plus a boolean leap-second
/// label. All public UTC methods account for this transparently; the
/// difference only matters when reasoning about internal representation.
///
/// ```rust,no_run
/// use tempoch_core::{Time, TT};
/// use qtty::Day;
///
/// let t = Time::<TT>::from_julian_days(Day::new(2_451_545.0)).unwrap();
/// let jd_back = t.julian_days();
/// ```
///
/// Axis conversions use `.to::<A2>()` for closed-form routes and
/// `.to_with::<A2>(&ctx)` for UT1 routes that need a [`TimeContext`].
#[repr(transparent)]
pub struct Time<A: Axis> {
    storage: Storage<A>,
}

impl<A: Axis> Copy for Time<A> {}
impl<A: Axis> Clone for Time<A> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

// `leap` is a civil presentation label, not physical instant identity. UTC
// storage records underlying TAI seconds; `leap` is dropped on any axis
// round-trip. Excluding it keeps `PartialEq` consistent with `PartialOrd`
// and prevents two values representing the same instant from comparing
// unequal due to differing labels.
impl<A: Axis> PartialEq for Time<A> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.storage.seconds == other.storage.seconds
    }
}

impl<A: Axis> PartialOrd for Time<A> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.storage.seconds.partial_cmp(&other.storage.seconds)
    }
}

impl<A: Axis> core::fmt::Debug for Time<A> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Time<{}>({:.6}s{})",
            A::NAME,
            self.storage.seconds / Second::new(1.0),
            if self.storage.leap { " [leap]" } else { "" }
        )
    }
}

// ── Internal constructors ─────────────────────────────────────────────────

impl<A: Axis> Time<A> {
    #[inline]
    pub(crate) const fn from_storage(storage: Storage<A>) -> Self {
        Self { storage }
    }

    #[inline]
    pub(crate) fn storage(&self) -> Storage<A> {
        self.storage
    }
}

// ── Construction and encoding (continuous axes) ───────────────────────────

impl<A: ContinuousAxis> Time<A> {
    /// Build a `Time<A>` from SI seconds since J2000 TT on axis `A`.
    ///
    /// Fails on non-finite input.
    #[inline]
    pub fn from_si_seconds(seconds: Second) -> Result<Self, ConversionError> {
        Ok(Self::from_storage(Storage::new(seconds)?))
    }

    /// SI seconds since J2000 TT on axis `A`.
    #[inline]
    pub fn si_seconds(self) -> Second {
        self.storage.seconds
    }

    /// Build a `Time<A>` from an absolute Julian Day number on axis `A`.
    ///
    /// Fails on non-finite input.
    #[inline]
    pub fn from_julian_days(jd: Day) -> Result<Self, ConversionError> {
        Ok(Self::from_storage(Storage::new(jd_to_j2000_seconds(jd))?))
    }

    /// Julian Day number on axis `A`.
    #[inline]
    pub fn julian_days(self) -> Day {
        j2000_seconds_to_jd(self.storage.seconds)
    }

    /// Build a `Time<A>` from a Modified Julian Day value on axis `A`.
    ///
    /// Fails on non-finite input.
    #[inline]
    pub fn from_modified_julian_days(mjd: Day) -> Result<Self, ConversionError> {
        Ok(Self::from_storage(Storage::new(mjd_to_j2000_seconds(mjd))?))
    }

    /// Modified Julian Day on axis `A`.
    #[inline]
    pub fn modified_julian_days(self) -> Day {
        j2000_seconds_to_mjd(self.storage.seconds)
    }
}

// ── Axis conversion: `to` (infallible routes) ─────────────────────────────

impl<A: Axis> Time<A> {
    /// Infallible axis conversion. Compiles only for pairs with a closed-form,
    /// context-free conversion (e.g. TT↔TAI, TT↔TDB, UTC↔TAI).
    ///
    /// For UT1 conversions that require a [`TimeContext`], use
    /// [`to_with`](Self::to_with) instead.
    #[allow(private_bounds)]
    #[inline]
    pub fn to<A2: Axis>(self) -> Time<A2>
    where
        A: InfallibleConvertible<A2>,
    {
        Time::from_storage(<A as InfallibleConvertible<A2>>::convert(self.storage))
    }
}

// ── Axis conversion: `to_with` (context-required) ────────────────────────

impl<A: Axis> Time<A> {
    /// Context-required axis conversion. Compiles only for UT1 routes that
    /// need a [`TimeContext`].
    #[allow(private_bounds)]
    #[inline]
    pub fn to_with<A2: Axis>(self, ctx: &TimeContext) -> Result<Time<A2>, ConversionError>
    where
        A: ContextConvertible<A2>,
    {
        Ok(Time::from_storage(
            <A as ContextConvertible<A2>>::convert_with(self.storage, ctx)?,
        ))
    }
}

// ── Arithmetic (continuous axes only) ─────────────────────────────────────

impl<A: ContinuousAxis> core::ops::Sub for Time<A> {
    type Output = Second;
    #[inline]
    fn sub(self, rhs: Self) -> Second {
        self.storage.seconds - rhs.storage.seconds
    }
}

impl<A: ContinuousAxis> core::ops::Add<Second> for Time<A> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Second) -> Self {
        Self::from_storage(Storage::new_unchecked(self.storage.seconds + rhs, false))
    }
}

impl<A: ContinuousAxis> core::ops::Sub<Second> for Time<A> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Second) -> Self {
        Self::from_storage(Storage::new_unchecked(self.storage.seconds - rhs, false))
    }
}

impl<A: ContinuousAxis> core::ops::AddAssign<Second> for Time<A> {
    #[inline]
    fn add_assign(&mut self, rhs: Second) {
        self.storage.seconds += rhs;
    }
}

impl<A: ContinuousAxis> core::ops::SubAssign<Second> for Time<A> {
    #[inline]
    fn sub_assign(&mut self, rhs: Second) {
        self.storage.seconds -= rhs;
    }
}

// Notice: no `Sub` / `Add<Second>` for UTC. That is deliberate (RFC §9).

#[cfg(test)]
mod tests {
    use super::super::axis::{TAI, TCB, TCG, TDB, TT};
    use super::super::error::ConversionError;
    use super::*;

    const SECONDS_PER_DAY: Second = Second::new(86_400.0);

    #[test]
    fn tt_tai_round_trip_exact() {
        let tt = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
        let tai = tt.to::<TAI>();
        let tt2 = tai.to::<TT>();
        assert_eq!(tt.si_seconds(), tt2.si_seconds());
        assert!((tai.si_seconds() - Second::new(-32.184)).abs() < Second::new(1e-15));
    }

    #[test]
    fn tt_tdb_round_trip_model_error() {
        let tt = Time::<TT>::from_si_seconds(Second::new(1_000_000.0)).unwrap();
        let tdb = tt.to::<TDB>();
        let tt2 = tdb.to::<TT>();
        assert!(
            (tt.si_seconds() - tt2.si_seconds()).abs() < Second::new(1e-6),
            "round-trip error {:?}",
            tt - tt2
        );
    }

    #[test]
    fn tt_tcg_rate_difference() {
        let tt0 = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
        let tt1 = Time::<TT>::from_si_seconds(SECONDS_PER_DAY).unwrap();
        let tcg0 = tt0.to::<TCG>();
        let tcg1 = tt1.to::<TCG>();
        let drift: Second = (tcg1 - tcg0) - SECONDS_PER_DAY;
        let l_g = 6.969_290_134e-10_f64;
        let expected: Second = SECONDS_PER_DAY * (l_g / (1.0 - l_g));
        assert!(
            (drift - expected).abs() < Second::new(1e-11),
            "drift = {:?}, expected = {:?}",
            drift,
            expected
        );
    }

    #[test]
    fn tdb_tcb_linear() {
        let tdb = Time::<TDB>::from_si_seconds(Second::new(1_000_000.0)).unwrap();
        let tcb = tdb.to::<TCB>();
        let tdb2 = tcb.to::<TDB>();
        assert!(
            (tdb.si_seconds() - tdb2.si_seconds()).abs() < Second::new(1e-6),
            "round-trip diff {:?}",
            tdb.si_seconds() - tdb2.si_seconds()
        );
    }

    #[test]
    fn julian_days_round_trip() {
        let jd = Day::new(2_451_545.0);
        let t = Time::<TT>::from_julian_days(jd).unwrap();
        assert_eq!(t.julian_days(), jd);
    }

    #[test]
    fn mjd_matches_jd_minus_offset() {
        let jd = Day::new(2_451_545.0);
        let t = Time::<TT>::from_julian_days(jd).unwrap();
        let expected_mjd = Day::new(2_451_545.0 - 2_400_000.5);
        assert!((t.modified_julian_days() - expected_mjd).abs() < Day::new(1e-9));
    }

    #[test]
    fn si_seconds_and_julian_days_consistent() {
        let t = Time::<TT>::from_julian_days(Day::new(2_451_545.5)).unwrap();
        assert!((t.si_seconds() - SECONDS_PER_DAY / 2.0).abs() < Second::new(1e-10));
    }

    #[test]
    fn arithmetic_in_seconds() {
        let a = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
        let b = a + Second::new(10.0);
        let diff: Second = b - a;
        assert_eq!(diff, Second::new(10.0));
    }

    #[test]
    fn nonfinite_rejected() {
        assert_eq!(
            Time::<TT>::from_si_seconds(Second::new(f64::NAN)).unwrap_err(),
            ConversionError::NonFinite
        );
        assert_eq!(
            Time::<TT>::from_si_seconds(Second::new(f64::INFINITY)).unwrap_err(),
            ConversionError::NonFinite
        );
    }
}
