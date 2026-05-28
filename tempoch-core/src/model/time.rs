// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! `Time<S, F>` — canonical instant with compensated precision and a format tag.

use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use crate::earth::context::TimeContext;
use crate::encoding::jd_to_julian_centuries;
use crate::format::{J2000s, TimeFormat};
use crate::foundation::error::ConversionError;
use crate::model::scale::conversion::{ContextScaleConvert, InfallibleScaleConvert};
use crate::model::scale::{CoordinateScale, Scale, TT, UTC};
use crate::model::target::{ContextConversionTarget, ConversionTarget, InfallibleConversionTarget};
use crate::{FormatForScale, InfallibleFormatForScale};
use affn::algebra::{Space, SplitPoint1, SplitQuantity};
use qtty::time::TimeUnit;
use qtty::unit::Second as SecondUnit;
use qtty::{Quantity, Second};

/// Split-axis scalars must not be NaN; ±∞ may be stored but many conversions still reject them.
#[inline]
fn coordinate_pair_ok(hi: f64, lo: f64) -> bool {
    !hi.is_nan() && !lo.is_nan()
}

#[derive(Copy, Clone)]
pub(crate) struct ScaleAxis<S: Scale>(PhantomData<fn() -> S>);

impl<S: Scale> fmt::Debug for ScaleAxis<S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ScaleAxis").field(&S::NAME).finish()
    }
}

impl<S: Scale> Space for ScaleAxis<S> {}

/// A point in time on scale `S`, tagged with external format phantom `F`.
///
/// The default `F` is [`J2000s`], so `Time<S>` in code is `Time<S, J2000s>`:
/// SI seconds since J2000.0 TT on the scale's coordinate axis.
///
/// Storage is always a compensated `(hi, lo)` pair of seconds. The format tag
/// does not duplicate storage; it only types the API (`raw()`, conversions, …).
///
/// # Preconditions
///
/// **NaN must never appear** in encoded scalars or storage components — behavior is undefined if it does.
/// **±∞** may be carried when callers use instants as sentinels; operations that require finite coordinates
/// (ΔT loops, UTC civil decoding, POSIX Unix mapping, …) may still return [`ConversionError::NonFinite`].
pub struct Time<S: Scale, F: TimeFormat = J2000s> {
    instant: SplitPoint1<ScaleAxis<S>, SecondUnit>,
    _fmt: PhantomData<fn() -> F>,
}

impl<S: Scale, F: TimeFormat> Copy for Time<S, F> {}

impl<S: Scale, F: TimeFormat> Clone for Time<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: Scale, F: TimeFormat> PartialEq for Time<S, F> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.split_seconds() == other.split_seconds()
    }
}

impl<S: Scale, F: TimeFormat> PartialOrd for Time<S, F> {
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

impl<S: Scale, F: TimeFormat> fmt::Debug for Time<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (hi, lo) = self.split_seconds();
        f.debug_struct("Time")
            .field("scale", &S::NAME)
            .field("format", &F::NAME)
            .field("hi_s", &hi)
            .field("lo_s", &lo)
            .finish()
    }
}

impl<S: CoordinateScale, F> fmt::Display for Time<S, F>
where
    F: InfallibleFormatForScale<S>,
    qtty::Quantity<F::Unit>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if F::NAME == J2000s::NAME {
            write!(f, "{} {:.9}", S::NAME, self.total_seconds().value())
        } else {
            fmt::Display::fmt(&F::from_time(*self), f)
        }
    }
}

impl fmt::Display for Time<UTC, crate::format::Unix> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.try_raw_with(&TimeContext::new()) {
            Ok(q) => fmt::Display::fmt(&q, f),
            Err(_) => f.write_str("Unix(<invalid for display>)"),
        }
    }
}

impl<S: CoordinateScale, F> fmt::LowerExp for Time<S, F>
where
    F: InfallibleFormatForScale<S>,
    qtty::Quantity<F::Unit>: fmt::LowerExp,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerExp::fmt(&F::from_time(*self), f)
    }
}

impl<S: CoordinateScale, F> fmt::UpperExp for Time<S, F>
where
    F: InfallibleFormatForScale<S>,
    qtty::Quantity<F::Unit>: fmt::UpperExp,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperExp::fmt(&F::from_time(*self), f)
    }
}

impl<S: Scale, F: TimeFormat> Time<S, F> {
    #[inline]
    pub(crate) fn from_split(hi: Second, lo: Second) -> Self {
        debug_assert!(
            coordinate_pair_ok(hi.value(), lo.value()),
            "time split pair must not contain NaN"
        );
        let instant = SplitPoint1::new(hi, lo);
        let (hi, lo) = instant.coordinate().pair();
        debug_assert!(
            coordinate_pair_ok(hi.value(), lo.value()),
            "time split pair must not contain NaN"
        );
        Self {
            instant,
            _fmt: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn try_from_split(hi: Second, lo: Second) -> Result<Self, ConversionError> {
        if coordinate_pair_ok(hi.value(), lo.value()) {
            Ok(Self::from_split(hi, lo))
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    /// Same instant, different format tag (zero cost).
    #[inline]
    pub fn reinterpret<G: TimeFormat>(self) -> Time<S, G> {
        Time {
            instant: self.instant,
            _fmt: PhantomData,
        }
    }

    /// SI J2000-second tagged view of the same instant.
    #[inline]
    pub fn to_j2000s(self) -> Time<S, J2000s> {
        self.reinterpret()
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

impl<S: CoordinateScale> Time<S, J2000s> {
    /// Build from J2000 TT seconds on the scale's coordinate axis.
    #[inline]
    pub fn from_raw_j2000_seconds(seconds: Second) -> Result<Self, ConversionError> {
        Self::try_from_split(seconds, Second::new(0.0))
    }

    /// Build from a split J2000-second pair on the scale's coordinate axis.
    #[inline]
    pub fn try_from_raw_j2000_seconds_split(
        hi: Second,
        lo: Second,
    ) -> Result<Self, ConversionError> {
        Self::try_from_split(hi, lo)
    }

    #[inline]
    pub(crate) fn raw_j2000_seconds(self) -> Second {
        self.total_seconds()
    }

    /// Shift this instant forward by a typed duration.
    #[inline]
    pub fn shifted_by<U>(self, delta: qtty::Quantity<U>) -> Self
    where
        U: TimeUnit,
    {
        self + delta
    }

    /// Shift this instant backward by a typed duration.
    #[inline]
    pub fn shifted_back_by<U>(self, delta: qtty::Quantity<U>) -> Self
    where
        U: TimeUnit,
    {
        self - delta
    }

    /// Duration from `other` to `self`.
    #[inline]
    pub fn duration_since(self, other: Self) -> Second {
        self - other
    }

    /// Duration from `self` to `other`.
    #[inline]
    pub fn duration_until(self, other: Self) -> Second {
        other - self
    }
}

impl<S: CoordinateScale, F: InfallibleFormatForScale<S>> Time<S, F> {
    /// Encoded scalar for this format (derived from split storage).
    #[inline]
    pub fn raw(self) -> Quantity<F::Unit> {
        F::from_time(self)
    }

    /// Alias for [`Self::raw`].
    #[inline]
    pub fn quantity(self) -> Quantity<F::Unit> {
        F::from_time(self)
    }
}

impl<S: CoordinateScale, F: TimeFormat> Time<S, F> {
    /// Exact-precision duration from `other` to `self`.
    ///
    /// Unlike the [`Sub`] implementation that returns a `Quantity<F::Unit>`
    /// (and therefore goes through `f64`), this method projects the difference
    /// into [`crate::ExactDuration`], which has 1 ns resolution.
    ///
    /// **Precision note:** `Time<S>` stores instants as a compensated split-f64
    /// pair. Near typical astronomy epochs (e.g. J2000 ± 50 years) the ULP of
    /// the high word is roughly 120–150 ns, so differences smaller than that
    /// may not round-trip exactly. For sub-microsecond precision on two instants
    /// that were originally constructed from the same `ExactDuration` arithmetic,
    /// the compensation pair reduces the error significantly, but this is not a
    /// guarantee of nanosecond parity for arbitrary instants.
    ///
    /// Returns [`crate::DurationError::Overflow`] only if the difference is
    /// outside the i128-nanosecond range (≈ ±5.4 × 10²¹ yr), which is unreachable
    /// for any physical astronomy use case.
    #[inline]
    pub fn diff_exact(self, other: Self) -> Result<crate::ExactDuration, crate::DurationError> {
        let delta: Second = self.instant - other.instant;
        crate::ExactDuration::try_from_quantity(delta)
    }

    /// Shift this instant by an [`crate::ExactDuration`], returning `Err` if the
    /// duration's seconds component exceeds the `i64` range (≈ ±292 billion years).
    ///
    /// **Precision note:** The duration is split into a whole-second component and
    /// a sub-second nanosecond remainder, each added to the compensated split-f64
    /// storage separately. The whole-second part is an integer `f64` (exact for
    /// `|seconds| < 2^53`). The nanosecond remainder crosses the split-f64 storage
    /// boundary and is therefore bounded by the documented split-f64 precision
    /// limits (ULP ≈ 120–150 ns near J2000 ± 50 years), so shifts smaller than
    /// that threshold may not alter the stored instant.
    #[inline]
    pub fn try_add_exact(
        self,
        delta: crate::ExactDuration,
    ) -> Result<Self, crate::foundation::duration::DurationError> {
        let (whole_secs, sub_nanos) = delta.as_seconds_i64_nanos_checked()?;
        let t = self.instant + Second::new(whole_secs as f64);
        Ok(Self {
            instant: t + Second::new(sub_nanos as f64 * 1e-9),
            _fmt: PhantomData,
        })
    }

    /// Shift this instant backward by an [`crate::ExactDuration`], returning `Err`
    /// if the duration's seconds component exceeds the `i64` range.
    ///
    /// See [`Self::try_add_exact`] for precision notes.
    #[inline]
    pub fn try_sub_exact(
        self,
        delta: crate::ExactDuration,
    ) -> Result<Self, crate::foundation::duration::DurationError> {
        let (whole_secs, sub_nanos) = delta.as_seconds_i64_nanos_checked()?;
        let t = self.instant - Second::new(whole_secs as f64);
        Ok(Self {
            instant: t - Second::new(sub_nanos as f64 * 1e-9),
            _fmt: PhantomData,
        })
    }

    /// Shift this instant by an [`crate::ExactDuration`].
    ///
    /// **Panics** if the duration's seconds component exceeds the `i64` range
    /// (≈ ±292 billion years). Use [`try_add_exact`](Self::try_add_exact) for
    /// the fallible variant that returns `Err` instead.
    ///
    /// See [`try_add_exact`](Self::try_add_exact) for precision notes.
    #[inline]
    pub fn add_exact(self, delta: crate::ExactDuration) -> Self {
        self.try_add_exact(delta)
            .expect("ExactDuration::add_exact: duration exceeds i64 seconds range")
    }

    /// Shift this instant backward by an [`crate::ExactDuration`].
    ///
    /// **Panics** if the duration's seconds component exceeds the `i64` range.
    /// Use [`try_sub_exact`](Self::try_sub_exact) for the fallible variant.
    ///
    /// See [`try_add_exact`](Self::try_add_exact) for precision notes.
    #[inline]
    pub fn sub_exact(self, delta: crate::ExactDuration) -> Self {
        self.try_sub_exact(delta)
            .expect("ExactDuration::sub_exact: duration exceeds i64 seconds range")
    }

    /// Round this instant to the nearest multiple of `quantum` measured from
    /// `epoch`. Banker's rounding (half-to-even) at the quantum boundary.
    /// Returns `self` unchanged on overflow.
    pub fn round_to_epoch(self, epoch: Self, quantum: crate::ExactDuration) -> Self {
        match self.diff_exact(epoch) {
            Ok(d) => epoch.add_exact(d.round_to(quantum)),
            Err(_) => self,
        }
    }

    /// Floor this instant toward `epoch − ∞` at `quantum`.
    pub fn floor_to_epoch(self, epoch: Self, quantum: crate::ExactDuration) -> Self {
        match self.diff_exact(epoch) {
            Ok(d) => epoch.add_exact(d.floor_to(quantum)),
            Err(_) => self,
        }
    }

    /// Ceil this instant toward `epoch + ∞` at `quantum`.
    pub fn ceil_to_epoch(self, epoch: Self, quantum: crate::ExactDuration) -> Self {
        match self.diff_exact(epoch) {
            Ok(d) => epoch.add_exact(d.ceil_to(quantum)),
            Err(_) => self,
        }
    }
}

impl<S: CoordinateScale, F> Time<S, F>
where
    F: FormatForScale<S>,
{
    #[inline]
    pub fn try_raw_with(self, ctx: &TimeContext) -> Result<Quantity<F::Unit>, ConversionError> {
        F::try_from_time(self, ctx)
    }
}

impl<S: Scale, F: TimeFormat> Time<S, F> {
    /// Unified infallible conversion to a scale/view target.
    #[allow(private_bounds)]
    #[inline]
    pub fn to<T>(self) -> T::Output
    where
        T: InfallibleConversionTarget<S, F>,
    {
        T::convert(self)
    }

    /// Unified fallible conversion to a scale/view target.
    #[allow(private_bounds)]
    #[inline]
    pub fn try_to<T>(self) -> Result<T::Output, ConversionError>
    where
        T: ConversionTarget<S, F>,
    {
        T::try_convert(self)
    }

    /// Unified context-backed conversion to a scale/view target.
    #[allow(private_bounds)]
    #[inline]
    pub fn to_with<T>(self, ctx: &TimeContext) -> Result<T::Output, ConversionError>
    where
        T: ContextConversionTarget<S, F>,
    {
        T::convert_with(self, ctx)
    }

    /// Infallible scale conversion; preserves format tag `F`.
    #[allow(private_bounds)]
    #[inline]
    pub fn to_scale<S2: Scale>(self) -> Time<S2, F>
    where
        S: InfallibleScaleConvert<S2>,
    {
        let (hi, lo) = self.split_seconds();
        let (new_hi, new_lo) = <S as InfallibleScaleConvert<S2>>::convert(hi, lo);
        Time::from_split(new_hi, new_lo)
    }

    /// Context-required scale conversion (UT1 routes); preserves `F`.
    #[allow(private_bounds)]
    #[inline]
    pub fn to_scale_with<S2: Scale>(self, ctx: &TimeContext) -> Result<Time<S2, F>, ConversionError>
    where
        S: ContextScaleConvert<S2>,
    {
        let (hi, lo) = self.split_seconds();
        let (new_hi, new_lo) = <S as ContextScaleConvert<S2>>::convert_with(hi, lo, ctx)?;
        Ok(Time::from_split(new_hi, new_lo))
    }
}

impl<S: Scale, F: FormatForScale<S>> Time<S, F> {
    /// Fallible constructor from an encoded scalar.
    ///
    /// Only surfaces **domain** failures from format decoding (UTC policy, leap seconds, ranges, …).
    /// Scalar hygiene is a caller precondition: **NaN must not be passed**; ±∞ is accepted only where the format decoder tolerates it.
    #[inline]
    pub fn try_new(raw: Quantity<F::Unit>) -> Result<Self, ConversionError> {
        F::try_into_time(raw, &TimeContext::new())
    }

    /// Like [`Self::try_new`], but uses `ctx` for UTC / POSIX decoding policy.
    #[inline]
    pub fn try_new_with(
        raw: Quantity<F::Unit>,
        ctx: &TimeContext,
    ) -> Result<Self, ConversionError> {
        F::try_into_time(raw, ctx)
    }
}

impl<S: Scale, F: InfallibleFormatForScale<S>> Time<S, F> {
    /// Infallible constructor from the raw scalar value for format `F`.
    ///
    /// # Panics
    ///
    /// If `value` is **NaN**. ±∞ is allowed as storage when callers use sentinel instants.
    #[track_caller]
    #[inline]
    pub fn new(value: f64) -> Self {
        assert!(
            !value.is_nan(),
            "time scalar must not be NaN (±∞ is allowed)"
        );
        F::into_time(Quantity::<F::Unit>::new(value))
    }
}

impl<S: CoordinateScale, F: InfallibleFormatForScale<S>> Time<S, F> {
    #[inline]
    pub fn min(self, other: Self) -> Self {
        if self <= other {
            self
        } else {
            other
        }
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        if self >= other {
            self
        } else {
            other
        }
    }

    #[inline]
    pub fn mean(self, other: Self) -> Self {
        let t = self.to_j2000s() + ((other.to_j2000s() - self.to_j2000s()) * 0.5);
        t.reinterpret()
    }
}

/// TT Julian date at J2000.0 (`JD 2 451 545.0`); matches [`Self::jd_epoch_tt`], usable in `const`.
impl Time<TT, crate::format::JD> {
    pub const JD_EPOCH_J2000_0: Self = Self {
        instant: SplitPoint1::from_split(SplitQuantity::from_normalized_parts(
            Second::new(0.0),
            Second::new(0.0),
        )),
        _fmt: PhantomData,
    };
}

impl<S: Scale> Time<S, crate::format::JD> {
    /// TT J2000.0 as a Julian Date on scale `S` (JD 2 451 545.0).
    #[inline]
    pub fn jd_epoch_tt() -> Self
    where
        S: CoordinateScale,
    {
        Time::<S, J2000s>::from_raw_j2000_seconds(Second::new(0.0))
            .expect("J2000 origin")
            .reinterpret()
    }

    #[inline]
    pub fn value(self) -> f64
    where
        S: CoordinateScale,
    {
        self.raw().value()
    }

    #[inline]
    pub fn julian_centuries(self) -> f64
    where
        S: CoordinateScale,
    {
        jd_to_julian_centuries(self.raw())
    }
}

impl<S: Scale> Time<S, crate::format::MJD> {
    #[inline]
    pub fn value(self) -> f64
    where
        S: CoordinateScale,
    {
        self.raw().value()
    }
}

impl<S: CoordinateScale, F, U> Add<Quantity<U>> for Time<S, F>
where
    F: InfallibleFormatForScale<S>,
    U: TimeUnit,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: Quantity<U>) -> Self::Output {
        Self {
            instant: self.instant + rhs.to::<SecondUnit>(),
            _fmt: PhantomData,
        }
    }
}

impl<S: CoordinateScale, F, U> Sub<Quantity<U>> for Time<S, F>
where
    F: InfallibleFormatForScale<S>,
    U: TimeUnit,
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Quantity<U>) -> Self::Output {
        Self {
            instant: self.instant - rhs.to::<SecondUnit>(),
            _fmt: PhantomData,
        }
    }
}

impl<S: CoordinateScale, F> Sub for Time<S, F>
where
    F: InfallibleFormatForScale<S>,
    F::Unit: TimeUnit,
{
    type Output = Quantity<F::Unit>;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        let delta: Second = self.instant - rhs.instant;
        delta.to::<F::Unit>()
    }
}

impl<S: CoordinateScale, F, U> AddAssign<Quantity<U>> for Time<S, F>
where
    F: InfallibleFormatForScale<S>,
    U: TimeUnit,
{
    #[inline]
    fn add_assign(&mut self, rhs: Quantity<U>) {
        *self = *self + rhs;
    }
}

impl<S: CoordinateScale, F, U> SubAssign<Quantity<U>> for Time<S, F>
where
    F: InfallibleFormatForScale<S>,
    U: TimeUnit,
{
    #[inline]
    fn sub_assign(&mut self, rhs: Quantity<U>) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::J2000s;
    use crate::foundation::duration::ExactDuration;
    use crate::model::scale::TAI;

    type TaiJ2000 = Time<TAI, J2000s>;

    fn j2000_tai() -> TaiJ2000 {
        TaiJ2000::from_raw_j2000_seconds(Second::new(0.0)).unwrap()
    }

    fn j2000_tai_plus_50yr() -> TaiJ2000 {
        // 50 Julian years = 50 * 365.25 * 86400 = 1_577_836_800 s
        TaiJ2000::from_raw_j2000_seconds(Second::new(1_577_836_800.0)).unwrap()
    }

    #[test]
    fn add_exact_1ns_at_j2000() {
        let t = j2000_tai();
        let d = ExactDuration::from_nanos(1);
        let shifted = t.add_exact(d);
        let diff = shifted.diff_exact(t).unwrap();
        // Near J2000 hi ≈ 0; lo stores the 1 ns shift exactly.
        assert_eq!(diff.as_nanos_i128(), 1, "1 ns shift at J2000 must be exact");
    }

    #[test]
    fn add_sub_round_trip_1ns_at_j2000_plus_50yr() {
        let t = j2000_tai_plus_50yr();
        // 1 ns: ULP of hi at 1.57e9 s is ~240 ns, so the lo word carries it.
        for ns in [1_i128, 123, 999] {
            let d = ExactDuration::from_nanos(ns);
            let shifted = t.add_exact(d).sub_exact(d);
            let back = shifted.diff_exact(t).unwrap();
            assert!(
                back.as_nanos_i128().abs() < 100,
                "add/sub round-trip drift at J2000+50yr for {ns} ns: {} ns",
                back.as_nanos_i128()
            );
        }
    }

    #[test]
    fn add_exact_1yr_plus_1ns_preserves_1ns() {
        let t = j2000_tai();
        // 1 Julian year = 31_557_600 s
        let one_year = ExactDuration::from_nanos(31_557_600 * 1_000_000_000);
        let one_ns = ExactDuration::from_nanos(1);
        let combined = (one_year + one_ns)
            .checked_add(ExactDuration::ZERO)
            .unwrap();
        let d_year = t.add_exact(one_year);
        let d_combined = t.add_exact(combined);
        let diff = d_combined.diff_exact(d_year).unwrap();
        // The difference should be 1 ns; allow up to 2 ns for sub-nanosecond f64 rounding.
        assert!(
            diff.as_nanos_i128().abs() <= 2,
            "1 yr + 1 ns shift must preserve 1 ns component; diff = {} ns",
            diff.as_nanos_i128()
        );
    }

    #[test]
    fn try_add_exact_overflow_returns_err() {
        let t = j2000_tai();
        // ExactDuration::MAX has > i64::MAX seconds → try_add_exact must return Err.
        let result = t.try_add_exact(ExactDuration::MAX);
        assert!(
            result.is_err(),
            "expected Err for try_add_exact(MAX), got Ok"
        );
        let result2 = t.try_sub_exact(ExactDuration::MAX);
        assert!(
            result2.is_err(),
            "expected Err for try_sub_exact(MAX), got Ok"
        );
    }

    #[test]
    #[should_panic(expected = "ExactDuration::add_exact")]
    fn add_exact_panics_on_overflow() {
        let t = j2000_tai();
        let _ = t.add_exact(ExactDuration::MAX);
    }
}
