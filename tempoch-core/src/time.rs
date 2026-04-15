// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! `Time<A, R = Native>` — the core public type.

use super::axis::Axis;
use super::constats::{J2000_JD_TT, JD_MINUS_MJD};
use super::context::TimeContext;
use super::conversion::{ContextConvertible, FallibleConvertible, InfallibleConvertible};
use super::error::ConversionError;
use super::representation::{JulianDays, ModifiedJulianDays, Native, Representation, SISeconds};
use super::storage::{ContinuousAxis, Storage};
use core::marker::PhantomData;
use qtty::{unit, Day, Second};

// ═══════════════════════════════════════════════════════════════════════════
// Time
// ═══════════════════════════════════════════════════════════════════════════

/// A point in time on axis `A`, encoded in representation `R` (default
/// `Native`).
///
/// Storage is private; this struct is `#[repr(transparent)]` over that
/// storage.
#[repr(transparent)]
pub struct Time<A: Axis, R: Representation<A> = Native> {
    storage: Storage<A>,
    _repr: PhantomData<fn() -> R>,
}

impl<A: Axis, R: Representation<A>> Copy for Time<A, R> {}
impl<A: Axis, R: Representation<A>> Clone for Time<A, R> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

// Don't derive `PartialEq` / `PartialOrd`: the target representation `R`
// participates in equality semantics. Two `Time<TT, JulianDays>` compare as
// expected; comparing `Time<TT, JulianDays>` with `Time<TT, SISeconds>`
// would be a type error, which is intentional.

impl<A: Axis, R: Representation<A>> PartialEq for Time<A, R> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.storage.seconds == other.storage.seconds && self.storage.leap == other.storage.leap
    }
}

impl<A: Axis, R: Representation<A>> PartialOrd for Time<A, R> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.storage.seconds.partial_cmp(&other.storage.seconds)
    }
}

impl<A: Axis, R: Representation<A>> core::fmt::Debug for Time<A, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Time<{}, {}>({}{})",
            A::NAME,
            R::NAME,
            self.storage.seconds.erase_unit_raw(),
            if self.storage.leap { " [leap]" } else { "" }
        )
    }
}

// ── Internal constructors ─────────────────────────────────────────────────

impl<A: Axis, R: Representation<A>> Time<A, R> {
    #[inline]
    pub(crate) const fn from_storage(storage: Storage<A>) -> Self {
        Self {
            storage,
            _repr: PhantomData,
        }
    }

    #[inline]
    #[doc(hidden)]
    pub(crate) fn storage(&self) -> Storage<A> {
        self.storage
    }
}

// ── Construction from SI seconds (continuous axes) ────────────────────────

impl<A: ContinuousAxis> Time<A, Native> {
    /// Build a `Time<A, Native>` from SI seconds since J2000 TT on axis `A`.
    ///
    /// Available only for continuous axes. Fails on non-finite input.
    #[inline]
    pub fn from_si_seconds(seconds: Second) -> Result<Self, ConversionError> {
        Ok(Self::from_storage(Storage::new(seconds)?))
    }

    /// SI seconds since J2000 TT on axis `A`.
    #[inline]
    pub fn si_seconds(self) -> Second {
        self.storage.seconds
    }
}

// ── Construction from Julian Day (continuous axes with JulianDays repr) ──

impl<A: Axis> Time<A, JulianDays>
where
    JulianDays: Representation<A>,
    A: ContinuousAxis,
{
    /// Build a `Time<A, JulianDays>` from an absolute Julian Day number on
    /// axis `A`. Fails on non-finite input.
    #[inline]
    pub fn from_julian_days(jd: Day) -> Result<Self, ConversionError> {
        Ok(Self::from_storage(Storage::new(
            (jd - J2000_JD_TT).to::<unit::Second>(),
        )?))
    }

    /// Julian Day number on axis `A`.
    #[inline]
    pub fn julian_days(self) -> Day {
        J2000_JD_TT + self.storage.seconds.to::<unit::Day>()
    }
}

impl<A: Axis> Time<A, ModifiedJulianDays>
where
    ModifiedJulianDays: Representation<A>,
    A: ContinuousAxis,
{
    /// Build a `Time<A, ModifiedJulianDays>` from an MJD value.
    #[inline]
    pub fn from_modified_julian_days(mjd: Day) -> Result<Self, ConversionError> {
        Ok(Self::from_storage(Storage::new(
            (mjd + JD_MINUS_MJD - J2000_JD_TT).to::<unit::Second>(),
        )?))
    }

    /// MJD value on axis `A`.
    #[inline]
    pub fn modified_julian_days(self) -> Day {
        J2000_JD_TT - JD_MINUS_MJD + self.storage.seconds.to::<unit::Day>()
    }
}

impl<A: Axis> Time<A, SISeconds>
where
    SISeconds: Representation<A>,
    A: ContinuousAxis,
{
    /// Build a `Time<A, SISeconds>` from SI seconds since J2000 TT.
    #[inline]
    pub fn from_seconds(seconds: Second) -> Result<Self, ConversionError> {
        Ok(Self::from_storage(Storage::new(seconds)?))
    }

    /// SI seconds since J2000 TT.
    #[inline]
    pub fn seconds(self) -> Second {
        self.storage.seconds
    }
}

// ── Representation transform (`repr`) ─────────────────────────────────────
//
// Representation is purely a type-level label on continuous axes (the
// stored scalar is the same). UTC's leap-aware representations require
// dedicated civil-time logic.

impl<A: Axis, R: Representation<A>> Time<A, R>
where
    A: ContinuousAxis,
{
    /// Same-axis representation change. Pure relabel on continuous axes.
    #[inline]
    pub fn repr<R2: Representation<A>>(self) -> Time<A, R2> {
        Time::from_storage(self.storage)
    }
}

// ── Axis conversion: `to` (infallible) ───────────────────────────────────

impl<A: Axis, R: Representation<A>> Time<A, R> {
    /// Infallible axis conversion. Compiles only for pairs that have a
    /// closed-form, context-free, always-succeeding conversion.
    #[inline]
    pub fn to<A2: Axis>(self) -> Time<A2, Native>
    where
        A: InfallibleConvertible<A2>,
        Native: Representation<A2>,
    {
        Time::from_storage(<A as InfallibleConvertible<A2>>::convert(self.storage))
    }
}

// ── Axis conversion: `try_to` (fallible) ──────────────────────────────────

impl<A: Axis, R: Representation<A>> Time<A, R> {
    /// Fallible axis conversion. Compiles only for pairs that depend on
    /// the compiled UTC–TAI history.
    #[inline]
    pub fn try_to<A2: Axis>(self) -> Result<Time<A2, Native>, ConversionError>
    where
        A: FallibleConvertible<A2>,
        Native: Representation<A2>,
    {
        Ok(Time::from_storage(
            <A as FallibleConvertible<A2>>::try_convert(self.storage)?,
        ))
    }
}

// ── Axis conversion: `to_with` (context-required) ────────────────────────

impl<A: Axis, R: Representation<A>> Time<A, R> {
    /// Context-required axis conversion. Compiles only for pairs that need
    /// a [`TimeContext`].
    #[inline]
    pub fn to_with<A2: Axis>(self, ctx: &TimeContext) -> Result<Time<A2, Native>, ConversionError>
    where
        A: ContextConvertible<A2>,
        Native: Representation<A2>,
    {
        Ok(Time::from_storage(
            <A as ContextConvertible<A2>>::convert_with(self.storage, ctx)?,
        ))
    }
}

// ── Arithmetic (continuous axes only) ─────────────────────────────────────

impl<A: Axis, R: Representation<A>> core::ops::Sub for Time<A, R>
where
    A: ContinuousAxis,
{
    type Output = Second;
    #[inline]
    fn sub(self, rhs: Self) -> Second {
        self.storage.seconds - rhs.storage.seconds
    }
}

impl<A: Axis, R: Representation<A>> core::ops::Add<Second> for Time<A, R>
where
    A: ContinuousAxis,
{
    type Output = Self;
    #[inline]
    fn add(self, rhs: Second) -> Self {
        Self::from_storage(Storage::new_unchecked(self.storage.seconds + rhs, false))
    }
}

impl<A: Axis, R: Representation<A>> core::ops::Sub<Second> for Time<A, R>
where
    A: ContinuousAxis,
{
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Second) -> Self {
        Self::from_storage(Storage::new_unchecked(self.storage.seconds - rhs, false))
    }
}

impl<A: Axis, R: Representation<A>> core::ops::AddAssign<Second> for Time<A, R>
where
    A: ContinuousAxis,
{
    #[inline]
    fn add_assign(&mut self, rhs: Second) {
        self.storage.seconds += rhs;
    }
}

impl<A: Axis, R: Representation<A>> core::ops::SubAssign<Second> for Time<A, R>
where
    A: ContinuousAxis,
{
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
    use super::super::representation::{JulianDays, ModifiedJulianDays, SISeconds};
    use super::*;

    // One day expressed as typed SI seconds.
    const SECONDS_PER_DAY: Second = Second::new(86_400.0);

    #[test]
    fn tt_tai_round_trip_exact() {
        let tt = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
        let tai = tt.to::<TAI>();
        let tt2 = tai.to::<TT>();
        assert_eq!(tt.si_seconds(), tt2.si_seconds());
        // TAI at J2000 TT should be −32.184 s on TAI axis.
        assert!((tai.si_seconds() - Second::new(-32.184)).abs() < Second::new(1e-15));
    }

    #[test]
    fn tt_tdb_round_trip_model_error() {
        // Pick a non-zero epoch so the periodic term is nonzero.
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
        // dTCG/dTT = 1 / (1 - L_G), so one TT-day's worth of TCG exceeds
        // one day by L_G / (1 - L_G) · 86400 ≈ 6.02e-5 s.
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
        // Pick a non-zero epoch so TDB0 term is visible.
        let tdb = Time::<TDB>::from_si_seconds(Second::new(1_000_000.0)).unwrap();
        let tcb = tdb.to::<TCB>();
        let tdb2 = tcb.to::<TDB>();
        // ULP-limited: t0 is ~7e8 s away, so 1 ULP ≈ 1e-7 s.
        assert!(
            (tdb.si_seconds() - tdb2.si_seconds()).abs() < Second::new(1e-6),
            "round-trip diff {:?}",
            tdb.si_seconds() - tdb2.si_seconds()
        );
    }

    #[test]
    fn julian_days_round_trip() {
        let jd = Day::new(2_451_545.0);
        let t: Time<TT, JulianDays> = Time::from_julian_days(jd).unwrap();
        assert_eq!(t.julian_days(), jd);
    }

    #[test]
    fn mjd_matches_jd_minus_offset() {
        let jd = Day::new(2_451_545.0);
        let t_jd: Time<TT, JulianDays> = Time::from_julian_days(jd).unwrap();
        let t_mjd: Time<TT, ModifiedJulianDays> = t_jd.repr();
        let expected_mjd = Day::new(2_451_545.0 - 2_400_000.5);
        assert!((t_mjd.modified_julian_days() - expected_mjd).abs() < Day::new(1e-9));
    }

    #[test]
    fn repr_transform_preserves_instant() {
        let t_jd: Time<TT, JulianDays> = Time::from_julian_days(Day::new(2_451_545.5)).unwrap();
        let t_si: Time<TT, SISeconds> = t_jd.repr();
        assert!((t_si.seconds() - SECONDS_PER_DAY / 2.0).abs() < Second::new(1e-10));
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
