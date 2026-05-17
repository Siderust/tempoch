// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! `Time<S>` — the core public type.

use core::marker::PhantomData;

use super::context::TimeContext;
use super::error::ConversionError;
use super::scale::conversion::{ContextScaleConvert, InfallibleScaleConvert};
use super::scale::{CoordinateScale, Scale};
use super::target::{ContextConversionTarget, ConversionTarget, InfallibleConversionTarget};
use affn::algebra::{Space, SplitPoint1};
use qtty::unit::Second as SecondUnit;
use qtty::Second;

#[inline]
fn is_finite_pair(hi: f64, lo: f64) -> bool {
    hi.is_finite() && lo.is_finite()
}

#[derive(Copy, Clone)]
pub(crate) struct ScaleAxis<S: Scale>(PhantomData<fn() -> S>);

impl<S: Scale> core::fmt::Debug for ScaleAxis<S> {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ScaleAxis").field(&S::NAME).finish()
    }
}

impl<S: Scale> Space for ScaleAxis<S> {}

/// A point in time on scale `S`.
///
/// Internally, `Time<S>` stores a compensated `(hi, lo)` pair of seconds since
/// J2000 TT on the scale's coordinate axis. The pair sums to the exact value
/// represented by the instance, while keeping the low-order remainder small
/// enough to retain much better precision than a single `f64`.
///
/// `Time<S>` is intentionally modeled as an affine point, not as a raw scalar:
/// subtracting two instants yields a duration, while adding or subtracting a
/// duration shifts an instant. Internally this is represented with
/// `affn::SplitPoint1`, which preserves the same point-vs-displacement
/// semantics used elsewhere in the codebase.
///
/// The split representation exists because astronomical epochs are large
/// values, while important corrections are often tiny. A single `f64` would
/// discard low-order precision too aggressively once epoch-sized values are
/// combined with sub-second or microsecond-scale offsets, so `Time<S>` keeps
/// the large component in `hi` and the residual correction in `lo`.
///
/// `UTC` remains special: it stores a continuous instant on the same internal
/// axis used by `TAI`, but its civil interpretation still comes from the
/// active UTC-TAI table. Raw JD/MJD/J2000-second helpers and second-based
/// arithmetic operate on that stored instant axis; use the civil API when you
/// need leap-second-labelled UTC values.
pub struct Time<S: Scale> {
    instant: SplitPoint1<ScaleAxis<S>, SecondUnit>,
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
        self.split_seconds() == other.split_seconds()
    }
}

impl<S: Scale> PartialOrd for Time<S> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        let (self_hi, self_lo) = self.split_seconds();
        let (other_hi, other_lo) = other.split_seconds();
        match self_hi.partial_cmp(&other_hi) {
            Some(core::cmp::Ordering::Equal) => self_lo.partial_cmp(&other_lo),
            ordering => ordering,
        }
    }
}

impl<S: Scale> core::fmt::Debug for Time<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let (hi, lo) = self.split_seconds();
        write!(f, "Time<{}>({:.17e}, {:.17e})", S::NAME, hi, lo)
    }
}

impl<S: Scale> core::fmt::Display for Time<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} {:.9}", S::NAME, self.total_seconds())
    }
}

impl<S: Scale> Time<S> {
    #[inline]
    pub(crate) fn new_unchecked(hi: Second, lo: Second) -> Self {
        debug_assert!(hi.is_finite());
        debug_assert!(lo.is_finite());
        let instant = SplitPoint1::new(hi, lo);
        let (hi, lo) = instant.coordinate().pair();
        debug_assert!(hi.is_finite());
        debug_assert!(lo.is_finite());
        Self { instant }
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
        self.instant.coordinate().pair()
    }

    #[inline]
    pub(crate) fn total_seconds(self) -> Second {
        self.instant.coordinate().total()
    }

    /// Raw internal storage pair in J2000-TT seconds on the instance scale.
    #[inline]
    pub fn raw_seconds_pair(self) -> (Second, Second) {
        self.split_seconds()
    }
}

impl<S: CoordinateScale> Time<S> {
    /// Build from J2000 TT seconds on the scale's coordinate axis.
    #[inline]
    pub(crate) fn from_raw_j2000_seconds(seconds: Second) -> Result<Self, ConversionError> {
        Self::try_new(seconds, Second::new(0.0))
    }

    /// Build from a split J2000-second pair on the scale's coordinate axis.
    ///
    /// This is the public constructor for FFI and wrapper layers that need to
    /// preserve the compensated `(hi, lo)` representation instead of
    /// flattening it to one scalar.
    #[inline]
    pub fn try_from_raw_j2000_seconds_split(
        hi: Second,
        lo: Second,
    ) -> Result<Self, ConversionError> {
        Self::try_new(hi, lo)
    }

    /// Build from a split J2000-second pair.
    #[inline]
    #[cfg(test)]
    pub(crate) fn from_raw_j2000_seconds_split(
        hi: Second,
        lo: Second,
    ) -> Result<Self, ConversionError> {
        Self::try_new(hi, lo)
    }

    /// Scale-coordinate seconds since J2000 TT.
    #[inline]
    pub(crate) fn raw_j2000_seconds(self) -> Second {
        self.total_seconds()
    }

    /// Shift this instant forward by a typed duration.
    ///
    /// This is a named shorthand for `self + delta`.
    #[inline]
    pub fn shifted_by<U>(self, delta: qtty::Quantity<U>) -> Self
    where
        U: qtty::time::TimeUnit,
    {
        self + delta
    }

    /// Shift this instant backward by a typed duration.
    ///
    /// This is a named shorthand for `self - delta`.
    #[inline]
    pub fn shifted_back_by<U>(self, delta: qtty::Quantity<U>) -> Self
    where
        U: qtty::time::TimeUnit,
    {
        self - delta
    }

    /// Duration from `other` to `self`.
    ///
    /// This is a named shorthand for `self - other`.
    #[inline]
    pub fn duration_since(self, other: Self) -> Second {
        self - other
    }

    /// Duration from `self` to `other`.
    ///
    /// This is a named shorthand for `other - self`.
    #[inline]
    pub fn duration_until(self, other: Self) -> Second {
        other - self
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

impl<S: CoordinateScale> core::ops::Sub for Time<S> {
    type Output = Second;

    #[inline]
    fn sub(self, rhs: Self) -> Second {
        self.instant - rhs.instant
    }
}

impl<S: CoordinateScale, U> core::ops::Add<qtty::Quantity<U>> for Time<S>
where
    U: qtty::time::TimeUnit,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: qtty::Quantity<U>) -> Self {
        self.add_seconds_internal(rhs.to::<qtty::unit::Second>())
    }
}

impl<S: CoordinateScale, U> core::ops::Sub<qtty::Quantity<U>> for Time<S>
where
    U: qtty::time::TimeUnit,
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: qtty::Quantity<U>) -> Self {
        self.sub_seconds_internal(rhs.to::<qtty::unit::Second>())
    }
}

impl<S: CoordinateScale, U> core::ops::AddAssign<qtty::Quantity<U>> for Time<S>
where
    U: qtty::time::TimeUnit,
{
    #[inline]
    fn add_assign(&mut self, rhs: qtty::Quantity<U>) {
        *self = self.add_seconds_internal(rhs.to::<qtty::unit::Second>());
    }
}

impl<S: CoordinateScale, U> core::ops::SubAssign<qtty::Quantity<U>> for Time<S>
where
    U: qtty::time::TimeUnit,
{
    #[inline]
    fn sub_assign(&mut self, rhs: qtty::Quantity<U>) {
        *self = self.sub_seconds_internal(rhs.to::<qtty::unit::Second>());
    }
}

impl<S: CoordinateScale> Time<S> {
    #[inline]
    fn add_seconds_internal(self, rhs: Second) -> Self {
        Self {
            instant: self.instant + rhs,
        }
    }

    #[inline]
    fn sub_seconds_internal(self, rhs: Second) -> Self {
        Self {
            instant: self.instant - rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::encoding::{j2000_seconds_to_mjd, mjd_to_j2000_seconds};
    use super::super::scale::{TAI, TCG, TDB, TT, UTC};
    use super::*;

    #[test]
    fn normalized_constructor_keeps_sum() {
        let time = Time::<TT>::from_raw_j2000_seconds_split(Second::new(1.0e9), Second::new(0.25))
            .unwrap();
        assert!((time.raw_j2000_seconds() - Second::new(1.0e9 + 0.25)).abs() < Second::new(1e-6));
    }

    #[test]
    fn tt_tai_round_trip_exact_offset() {
        let tt = Time::<TT>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
        let tai = tt.to_scale::<TAI>();
        let roundtrip = tai.to_scale::<TT>();
        assert!(
            (tt.raw_j2000_seconds() - roundtrip.raw_j2000_seconds()).abs() < Second::new(1e-12)
        );
        assert!((tai.raw_j2000_seconds() - Second::new(-32.184)).abs() < Second::new(1e-12));
    }

    #[test]
    fn tt_tdb_round_trip_model_error() {
        let tt = Time::<TT>::from_raw_j2000_seconds(Second::new(1_000_000.0)).unwrap();
        let tdb = tt.to_scale::<TDB>();
        let tt2 = tdb.to_scale::<TT>();
        assert!((tt.raw_j2000_seconds() - tt2.raw_j2000_seconds()).abs() < Second::new(1e-6));
    }

    #[test]
    fn tt_tcg_offset_is_finite() {
        let tt = Time::<TT>::from_raw_j2000_seconds(qtty::Day::new(1.0).to::<qtty::unit::Second>())
            .unwrap();
        let tcg = tt.to_scale::<TCG>();
        assert!(tcg.raw_j2000_seconds().is_finite());
    }

    #[test]
    fn utc_exposes_raw_axis_helpers_and_arithmetic() {
        let utc =
            Time::<UTC>::from_raw_j2000_seconds(mjd_to_j2000_seconds(qtty::Day::new(51_544.5)))
                .unwrap();
        let shifted = utc + Second::new(10.0);
        assert_eq!(
            j2000_seconds_to_mjd(utc.raw_j2000_seconds()),
            qtty::Day::new(51_544.5)
        );
        assert!((shifted - utc - Second::new(10.0)).abs() < Second::new(1e-12));
    }

    #[test]
    fn add_days_and_seconds_to_time() {
        let t = Time::<TT>::from_raw_j2000_seconds(Second::new(1_000_000.0)).unwrap();
        let t_plus_day = t + qtty::Day::new(1.0);
        let t_plus_seconds = t + Second::new(86_400.0);
        assert!(
            (t_plus_day.raw_j2000_seconds() - t_plus_seconds.raw_j2000_seconds()).abs()
                < Second::new(1e-12)
        );
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn scale_axis_debug_and_time_formatting_are_stable() {
        let axis = ScaleAxis::<TT>(PhantomData);
        assert_eq!(format!("{axis:?}"), "ScaleAxis(\"TT\")");

        let time = Time::<TT>::from_raw_j2000_seconds(Second::new(1.25)).unwrap();
        let cloned = time.clone();
        assert_eq!(cloned, time);
        assert!(format!("{time:?}").starts_with("Time<TT>("));
        assert_eq!(format!("{time}"), "TT 1.250000000 s");
    }

    #[test]
    fn time_partial_order_and_assign_arithmetic() {
        let start = Time::<TT>::from_raw_j2000_seconds(Second::new(10.0)).unwrap();
        let mut shifted = start;

        shifted += Second::new(3.0);
        assert!(shifted > start);
        assert_eq!(shifted - start, Second::new(3.0));

        shifted -= Second::new(1.25);
        assert_eq!(shifted - start, Second::new(1.75));
        assert_eq!(shifted - Second::new(1.75), start);
    }

    #[test]
    fn raw_j2000_constructor_rejects_nonfinite() {
        assert!(matches!(
            Time::<TT>::from_raw_j2000_seconds(Second::new(f64::NAN)),
            Err(ConversionError::NonFinite)
        ));
    }

    #[test]
    fn named_format_helpers_match_generic_conversion_targets() {
        let tt = Time::<TT>::from_raw_j2000_seconds(Second::new(12_345.0)).unwrap();
        assert_eq!(tt.shifted_by(Second::new(3.0)), tt + Second::new(3.0));
        assert_eq!(tt.shifted_back_by(Second::new(3.0)), tt - Second::new(3.0));
        assert_eq!(
            tt.duration_since(tt.shifted_back_by(Second::new(1.0))),
            Second::new(1.0)
        );
        assert_eq!(
            tt.duration_until(tt.shifted_by(Second::new(1.0))),
            Second::new(1.0)
        );

    }
}
