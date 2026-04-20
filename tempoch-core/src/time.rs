// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! `Time<S>` — the core public type.

use core::marker::PhantomData;

use super::context::TimeContext;
use super::encoding::{j2000_seconds_to_jd, j2000_seconds_to_mjd, jd_to_j2000_seconds, mjd_to_j2000_seconds};
use super::error::ConversionError;
use super::scale::{ContinuousScale, Scale};
use super::scale::conversion::{ContextScaleConvert, InfallibleScaleConvert};
use super::target::{ContextConversionTarget, ConversionTarget, InfallibleConversionTarget};
use qtty::time::Seconds;
use qtty::{Day, Second};

#[inline]
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    let err = (a - (s - bb)) + (b - bb);
    (s, err)
}

#[inline]
fn normalize_pair(hi: f64, lo: f64) -> (f64, f64) {
    let (sum, err) = two_sum(hi, lo);
    let (sum2, err2) = two_sum(sum, err);
    (sum2, err2)
}

#[inline]
fn is_finite_pair(hi: f64, lo: f64) -> bool {
    hi.is_finite() && lo.is_finite()
}

/// A point in time on scale `S`.
///
/// Internally, `Time<S>` stores a compensated `(hi, lo)` pair of seconds since
/// J2000 TT on the scale's coordinate axis. The pair sums to the exact value
/// represented by the instance, while keeping the low-order remainder small
/// enough to retain much better precision than a single `f64`.
///
/// `UTC` remains special: it stores a continuous instant on the same internal
/// axis used by `TAI`, and therefore does not expose raw JD/MJD/J2000-second
/// constructors or accessors. Use the civil API on `Time<UTC>` instead.
pub struct Time<S: Scale> {
    hi: Second,
    lo: Second,
    _scale: PhantomData<S>,
}

impl<S: Scale> Copy for Time<S> {}
impl<S: Scale> Clone for Time<S> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: Scale> PartialEq for Time<S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hi == other.hi && self.lo == other.lo
    }
}

impl<S: Scale> PartialOrd for Time<S> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        match self.hi.partial_cmp(&other.hi) {
            Some(core::cmp::Ordering::Equal) => self.lo.partial_cmp(&other.lo),
            ordering => ordering,
        }
    }
}

impl<S: Scale> core::fmt::Debug for Time<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Time<{}>({:.17e} s, {:.17e} s)",
            S::NAME,
            self.hi.value(),
            self.lo.value()
        )
    }
}

impl<S: Scale> core::fmt::Display for Time<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} {:.9} s", S::NAME, self.total_seconds().value())
    }
}

impl<S: Scale> Time<S> {
    #[inline]
    pub(crate) fn new_unchecked(hi: Second, lo: Second) -> Self {
        let (hi, lo) = normalize_pair(hi.value(), lo.value());
        Self {
            hi: Second::new(hi),
            lo: Second::new(lo),
            _scale: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn try_new(hi: Second, lo: Second) -> Result<Self, ConversionError> {
        if is_finite_pair(hi.value(), lo.value()) {
            Ok(Self::new_unchecked(hi, lo))
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    #[inline]
    pub(crate) fn split_seconds(self) -> (Second, Second) {
        (self.hi, self.lo)
    }

    #[inline]
    pub(crate) fn total_seconds(self) -> Second {
        self.hi + self.lo
    }

    /// Raw internal storage pair in J2000-TT seconds on the instance scale.
    #[inline]
    pub fn raw_seconds_pair(self) -> (Second, Second) {
        self.split_seconds()
    }
}

impl<S: ContinuousScale> Time<S> {
    /// Build from J2000 TT seconds on the scale's coordinate axis.
    #[inline]
    pub fn from_j2000_seconds(seconds: Seconds) -> Result<Self, ConversionError> {
        Self::try_new(seconds, Second::new(0.0))
    }

    /// Build from a split J2000-second pair.
    #[inline]
    pub fn from_j2000_seconds_split(hi: Seconds, lo: Seconds) -> Result<Self, ConversionError> {
        Self::try_new(hi, lo)
    }

    /// Build from a Julian Day value on the scale's coordinate axis.
    #[inline]
    pub fn from_julian_days(jd: Day) -> Result<Self, ConversionError> {
        if !jd.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        Self::from_j2000_seconds(jd_to_j2000_seconds(jd))
    }

    /// Build from a Modified Julian Day value on the scale's coordinate axis.
    #[inline]
    pub fn from_modified_julian_days(mjd: Day) -> Result<Self, ConversionError> {
        if !mjd.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        Self::from_j2000_seconds(mjd_to_j2000_seconds(mjd))
    }

    /// Scale-coordinate seconds since J2000 TT.
    #[inline]
    pub fn j2000_seconds(self) -> Seconds {
        self.total_seconds()
    }

    /// Scale-coordinate Julian Day.
    #[inline]
    pub fn julian_days(self) -> Day {
        j2000_seconds_to_jd(self.total_seconds())
    }

    /// Scale-coordinate Modified Julian Day.
    #[inline]
    pub fn modified_julian_days(self) -> Day {
        j2000_seconds_to_mjd(self.total_seconds())
    }
}

impl<S: ContinuousScale> From<Second> for Time<S> {
    #[inline]
    fn from(value: Second) -> Self {
        Self::new_unchecked(value, Second::new(0.0))
    }
}

impl<S: ContinuousScale> From<f64> for Time<S> {
    #[inline]
    fn from(value: f64) -> Self {
        Self::new_unchecked(Second::new(value), Second::new(0.0))
    }
}

impl<S: Scale> Time<S> {
    /// Unified infallible conversion to a scale/view target.
    #[allow(private_bounds)]
    #[inline]
    pub fn to<T>(self) -> T::Output
    where
        T: InfallibleConversionTarget<S>,
    {
        T::convert(self)
    }

    /// Unified fallible conversion to a scale/view target.
    #[allow(private_bounds)]
    #[inline]
    pub fn try_to<T>(self) -> Result<T::Output, ConversionError>
    where
        T: ConversionTarget<S>,
    {
        T::try_convert(self)
    }

    /// Unified context-backed conversion to a scale/view target.
    #[allow(private_bounds)]
    #[inline]
    pub fn to_with<T>(self, ctx: &TimeContext) -> Result<T::Output, ConversionError>
    where
        T: ContextConversionTarget<S>,
    {
        T::convert_with(self, ctx)
    }

    /// Infallible scale conversion. Compiles only for pairs with a
    /// closed-form, context-free conversion.
    #[allow(private_bounds)]
    #[inline]
    pub fn to_scale<S2: Scale>(self) -> Time<S2>
    where
        S: InfallibleScaleConvert<S2>,
    {
        let (hi, lo) = self.split_seconds();
        let (new_hi, new_lo) = <S as InfallibleScaleConvert<S2>>::convert(hi, lo);
        Time::new_unchecked(new_hi, new_lo)
    }

    /// Context-required scale conversion (UT1 routes).
    #[allow(private_bounds)]
    #[inline]
    pub fn to_scale_with<S2: Scale>(self, ctx: &TimeContext) -> Result<Time<S2>, ConversionError>
    where
        S: ContextScaleConvert<S2>,
    {
        let (hi, lo) = self.split_seconds();
        let (new_hi, new_lo) = <S as ContextScaleConvert<S2>>::convert_with(hi, lo, ctx)?;
        Ok(Time::new_unchecked(new_hi, new_lo))
    }
}

impl<S: ContinuousScale> core::ops::Sub for Time<S> {
    type Output = Second;

    #[inline]
    fn sub(self, rhs: Self) -> Second {
        (self.hi - rhs.hi) + (self.lo - rhs.lo)
    }
}

impl<S: ContinuousScale> core::ops::Add<Second> for Time<S> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Second) -> Self {
        Self::new_unchecked(self.hi, self.lo + rhs)
    }
}

impl<S: ContinuousScale> core::ops::Sub<Second> for Time<S> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Second) -> Self {
        Self::new_unchecked(self.hi, self.lo - rhs)
    }
}

impl<S: ContinuousScale> core::ops::AddAssign<Second> for Time<S> {
    #[inline]
    fn add_assign(&mut self, rhs: Second) {
        *self = *self + rhs;
    }
}

impl<S: ContinuousScale> core::ops::SubAssign<Second> for Time<S> {
    #[inline]
    fn sub_assign(&mut self, rhs: Second) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::super::scale::{TAI, TCG, TDB, TT};
    use super::*;

    #[test]
    fn normalized_constructor_keeps_sum() {
        let time = Time::<TT>::from_j2000_seconds_split(Second::new(1.0e9), Second::new(0.25)).unwrap();
        assert!((time.j2000_seconds() - Second::new(1.0e9 + 0.25)).abs() < Second::new(1e-6));
    }

    #[test]
    fn tt_tai_round_trip_exact_offset() {
        let tt = Time::<TT>::from_j2000_seconds(Second::new(0.0)).unwrap();
        let tai = tt.to_scale::<TAI>();
        let roundtrip = tai.to_scale::<TT>();
        assert!((tt.j2000_seconds() - roundtrip.j2000_seconds()).abs() < Second::new(1e-12));
        assert!((tai.j2000_seconds() - Second::new(-32.184)).abs() < Second::new(1e-12));
    }

    #[test]
    fn tt_tdb_round_trip_model_error() {
        let tt = Time::<TT>::from_j2000_seconds(Second::new(1_000_000.0)).unwrap();
        let tdb = tt.to_scale::<TDB>();
        let tt2 = tdb.to_scale::<TT>();
        assert!((tt.j2000_seconds() - tt2.j2000_seconds()).abs() < Second::new(1e-6));
    }

    #[test]
    fn tt_tcg_offset_is_finite() {
        let tt = Time::<TT>::from_j2000_seconds(Second::new(86_400.0)).unwrap();
        let tcg = tt.to_scale::<TCG>();
        assert!(tcg.j2000_seconds().is_finite());
    }
}
