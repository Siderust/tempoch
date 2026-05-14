// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed encoded formats of [`crate::Time`].
//!
//! A *format* marker specifies how a time instant is externally expressed:
//! Julian Day (`JD`), Modified Julian Day (`MJD`), J2000 seconds (`J2000s`),
//! POSIX seconds (`Unix`), or GPS seconds (`GPS`). Format is orthogonal to
//! *scale*: `JulianDate<TT>` and `JulianDate<UTC>` share the same format but
//! live on different physical time axes, and the compiler treats them as
//! distinct, incompatible types.
//!
//! # Main types
//!
//! - [`TimeFormat`] — sealed marker trait for format tags (`JD`, `MJD`, …).
//! - [`EncodedTime<S, F>`](crate::EncodedTime) — a typed encoded instant; `S`
//!   is the [`Scale`] and `F` is the [`TimeFormat`].
//! - [`FormatForScale<S>`] — witness that format `F` can encode scale `S`.
//! - [`InfallibleFormatForScale<S>`] — witness that the round-trip is
//!   context-free.

use core::fmt;
use core::marker::PhantomData;

use crate::context::TimeContext;
use crate::encoding::{
    j2000_seconds_to_jd, j2000_seconds_to_mjd, jd_to_j2000_seconds, mjd_to_j2000_seconds,
};
use crate::error::ConversionError;
use crate::scale::conversion::InfallibleScaleConvert;
use crate::scale::{CoordinateScale, Scale, TAI, UTC};
use crate::sealed::Sealed;
use crate::target::{ContextConversionTarget, ConversionTarget, InfallibleConversionTarget};
use crate::time::Time;
use qtty::{Day, Quantity, Second, Unit};

/// Marker trait for an external time encoding such as JD or Unix time.
///
/// A `TimeFormat` value is a zero-sized tag that identifies how a time instant
/// is expressed (Julian Day, Modified Julian Day, J2000 seconds, POSIX seconds,
/// GPS seconds). It is orthogonal to [`Scale`], which identifies the physical
/// time axis.
///
/// Sealed: implementations live in this crate only.
#[allow(private_bounds)]
pub trait TimeFormat: Sealed + Copy + Clone + fmt::Debug + 'static {
    /// Quantity unit used by this format.
    type Unit: Unit;

    /// Human-readable format name.
    const NAME: &'static str;
}

/// Witness that format `F` can encode and decode instants on scale `S`.
#[allow(private_bounds)]
pub trait FormatForScale<S: Scale>: TimeFormat + Sealed {
    fn try_from_time(
        time: Time<S>,
        ctx: &TimeContext,
    ) -> Result<Quantity<Self::Unit>, ConversionError>;
    fn try_into_time(
        raw: Quantity<Self::Unit>,
        ctx: &TimeContext,
    ) -> Result<Time<S>, ConversionError>;
}

/// Witness that format `F` can encode scale `S` without a [`TimeContext`].
#[allow(private_bounds)]
pub trait InfallibleFormatForScale<S: Scale>: FormatForScale<S> + Sealed {
    fn from_time(time: Time<S>) -> Quantity<Self::Unit>;
    fn into_time(raw: Quantity<Self::Unit>) -> Time<S>;
}

// ── Format markers ────────────────────────────────────────────────────────────

/// J2000 seconds on the source scale's coordinate axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct J2000s;

/// Julian Day on the source scale's coordinate axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JD;

/// Modified Julian Day on the source scale's coordinate axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MJD;

/// POSIX seconds on the UTC civil axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unix;

/// GPS seconds since the GPS epoch on the TAI/GPS continuous axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GPS;

impl Sealed for J2000s {}
impl Sealed for JD {}
impl Sealed for MJD {}
impl Sealed for Unix {}
impl Sealed for GPS {}

impl TimeFormat for J2000s {
    type Unit = qtty::unit::Second;
    const NAME: &'static str = "J2000s";
}

impl TimeFormat for JD {
    type Unit = qtty::unit::Day;
    const NAME: &'static str = "JD";
}

impl TimeFormat for MJD {
    type Unit = qtty::unit::Day;
    const NAME: &'static str = "MJD";
}

impl TimeFormat for Unix {
    type Unit = qtty::unit::Second;
    const NAME: &'static str = "Unix";
}

impl TimeFormat for GPS {
    type Unit = qtty::unit::Second;
    const NAME: &'static str = "GPS";
}

// ── EncodedTime ───────────────────────────────────────────────────────────────

/// A typed external encoding of a [`Time<S>`] instant.
///
/// `EncodedTime<S, F>` carries two phantom type parameters:
///
/// - `S: Scale` — the physical time axis (`TT`, `TAI`, `UTC`, …).
/// - `F: TimeFormat` — the encoding scheme (`JD`, `MJD`, `J2000s`, `Unix`,
///   `GPS`).
///
/// The compiler therefore treats `EncodedTime<TT, JD>` and
/// `EncodedTime<UTC, JD>` as completely distinct, incompatible types even
/// though both carry a day-valued quantity internally.
pub struct EncodedTime<S: Scale, F: TimeFormat> {
    raw: Quantity<F::Unit>,
    _marker: PhantomData<fn() -> S>,
}

impl<S: Scale, F: TimeFormat> Copy for EncodedTime<S, F> {}

impl<S: Scale, F: TimeFormat> Clone for EncodedTime<S, F> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: Scale, F: TimeFormat> fmt::Debug for EncodedTime<S, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncodedTime")
            .field("scale", &S::NAME)
            .field("format", &F::NAME)
            .field("raw", &self.raw)
            .finish()
    }
}

impl<S: Scale, F: TimeFormat> fmt::Display for EncodedTime<S, F>
where
    qtty::Quantity<F::Unit>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.raw, f)
    }
}

impl<S: Scale, F: TimeFormat> fmt::LowerExp for EncodedTime<S, F>
where
    qtty::Quantity<F::Unit>: fmt::LowerExp,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerExp::fmt(&self.raw, f)
    }
}

impl<S: Scale, F: TimeFormat> fmt::UpperExp for EncodedTime<S, F>
where
    qtty::Quantity<F::Unit>: fmt::UpperExp,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperExp::fmt(&self.raw, f)
    }
}

impl<S: Scale, F: TimeFormat> PartialEq for EncodedTime<S, F> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<S: Scale, F: TimeFormat> PartialOrd for EncodedTime<S, F> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.raw.partial_cmp(&other.raw)
    }
}

impl<S: Scale, F: TimeFormat> EncodedTime<S, F> {
    #[inline]
    pub(crate) const fn new_unchecked(raw: Quantity<F::Unit>) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Construct from a raw typed quantity without checking for finiteness.
    ///
    /// For use in `const` contexts. The caller must ensure `raw` is finite;
    /// passing a non-finite value produces incorrect behaviour.
    #[inline]
    pub const fn from_raw_unchecked(raw: Quantity<F::Unit>) -> Self {
        Self::new_unchecked(raw)
    }

    /// Return the underlying typed quantity.
    #[inline]
    pub const fn raw(self) -> Quantity<F::Unit> {
        self.raw
    }

    /// Alias for [`Self::raw`].
    #[inline]
    pub const fn quantity(self) -> Quantity<F::Unit> {
        self.raw
    }
}

impl<S: Scale> EncodedTime<S, JD> {
    /// J2000.0 epoch as a Julian Date on scale `S` (JD 2 451 545.0).
    pub const J2000: Self = Self::from_raw_unchecked(crate::constats::J2000_JD_TT.raw());
}

impl<S: Scale, F> EncodedTime<S, F>
where
    F: FormatForScale<S>,
{
    /// Construct a typed encoded instant from its raw quantity.
    #[inline]
    pub fn try_new(raw: Quantity<F::Unit>) -> Result<Self, ConversionError> {
        if raw.is_finite() {
            Ok(Self::new_unchecked(raw))
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    /// Convert this encoded instant to the canonical [`Time<S>`] model.
    ///
    /// Snapshots the active time-data bundle at call time via
    /// [`TimeContext::new`]. For reproducible pipelines, prefer
    /// [`to_time_with`](Self::to_time_with) with an explicit context.
    #[inline]
    pub fn try_to_time(self) -> Result<Time<S>, ConversionError> {
        F::try_into_time(self.raw, &TimeContext::new())
    }

    /// Convert this encoded instant to the canonical [`Time<S>`] model using an explicit context.
    #[inline]
    pub fn to_time_with(self, ctx: &TimeContext) -> Result<Time<S>, ConversionError> {
        F::try_into_time(self.raw, ctx)
    }
}

impl<S: Scale, F> EncodedTime<S, F>
where
    F: InfallibleFormatForScale<S>,
{
    #[inline]
    pub(crate) fn from_time_infallible(time: Time<S>) -> Self {
        Self::new_unchecked(F::from_time(time))
    }

    /// Infallible conversion to the canonical [`Time<S>`] model.
    #[inline]
    pub fn to_time(self) -> Time<S> {
        F::into_time(self.raw)
    }

    /// Unified infallible conversion to a target scale or encoded format.
    #[allow(private_bounds)]
    #[inline]
    pub fn to<T>(self) -> T::Output
    where
        T: InfallibleConversionTarget<S>,
    {
        T::convert(self.to_time())
    }

    /// Unified fallible conversion to a target scale or encoded format.
    #[allow(private_bounds)]
    #[inline]
    pub fn try_to<T>(self) -> Result<T::Output, ConversionError>
    where
        T: ConversionTarget<S>,
    {
        T::try_convert(self.to_time())
    }
}

impl<S: Scale, F> EncodedTime<S, F>
where
    F: FormatForScale<S>,
{
    /// Unified context-backed conversion to a target scale or encoded format.
    #[allow(private_bounds)]
    #[inline]
    pub fn to_with<T>(self, ctx: &TimeContext) -> Result<T::Output, ConversionError>
    where
        T: ContextConversionTarget<S>,
    {
        T::convert_with(self.to_time_with(ctx)?, ctx)
    }
}

// ── Type aliases ──────────────────────────────────────────────────────────────

/// `EncodedTime<S, JD>` convenience alias.
pub type JulianDate<S> = EncodedTime<S, JD>;

/// `EncodedTime<S, MJD>` convenience alias.
pub type ModifiedJulianDate<S> = EncodedTime<S, MJD>;

/// `EncodedTime<S, J2000s>` convenience alias.
pub type J2000Seconds<S> = EncodedTime<S, J2000s>;

/// `EncodedTime<UTC, Unix>` convenience alias.
pub type UnixTime = EncodedTime<UTC, Unix>;

/// `EncodedTime<TAI, GPS>` convenience alias.
pub type GpsTime = EncodedTime<TAI, GPS>;

// ── FormatForScale impls ──────────────────────────────────────────────────────

macro_rules! coordinate_format {
    ($fmt:ty, $quantity:ty, $from_time:expr, $to_time:expr) => {
        impl<S: CoordinateScale> FormatForScale<S> for $fmt {
            #[inline]
            fn try_from_time(
                time: Time<S>,
                _ctx: &TimeContext,
            ) -> Result<$quantity, ConversionError> {
                Ok(<Self as InfallibleFormatForScale<S>>::from_time(time))
            }

            #[inline]
            fn try_into_time(
                raw: $quantity,
                _ctx: &TimeContext,
            ) -> Result<Time<S>, ConversionError> {
                Ok(<Self as InfallibleFormatForScale<S>>::into_time(raw))
            }
        }

        impl<S: CoordinateScale> InfallibleFormatForScale<S> for $fmt {
            #[inline]
            fn from_time(time: Time<S>) -> $quantity {
                $from_time(time)
            }

            #[inline]
            fn into_time(raw: $quantity) -> Time<S> {
                $to_time(raw)
            }
        }
    };
}

coordinate_format!(
    J2000s,
    Second,
    |time: Time<_>| time.raw_j2000_seconds(),
    |raw: Second| Time::from_raw_j2000_seconds(raw).expect("finite J2000 seconds must decode")
);
coordinate_format!(
    JD,
    Day,
    |time: Time<_>| j2000_seconds_to_jd(time.raw_j2000_seconds()),
    |raw: Day| Time::from_raw_j2000_seconds(jd_to_j2000_seconds(raw))
        .expect("finite Julian date must decode")
);
coordinate_format!(
    MJD,
    Day,
    |time: Time<_>| j2000_seconds_to_mjd(time.raw_j2000_seconds()),
    |raw: Day| Time::from_raw_j2000_seconds(mjd_to_j2000_seconds(raw))
        .expect("finite Modified Julian date must decode")
);

impl FormatForScale<UTC> for Unix {
    #[inline]
    fn try_from_time(time: Time<UTC>, ctx: &TimeContext) -> Result<Second, ConversionError> {
        time.raw_unix_seconds_with(ctx)
    }

    #[inline]
    fn try_into_time(raw: Second, ctx: &TimeContext) -> Result<Time<UTC>, ConversionError> {
        Time::from_raw_unix_seconds_with(raw, ctx)
    }
}

impl FormatForScale<TAI> for GPS {
    #[inline]
    fn try_from_time(time: Time<TAI>, _ctx: &TimeContext) -> Result<Second, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<TAI>>::from_time(time))
    }

    #[inline]
    fn try_into_time(raw: Second, _ctx: &TimeContext) -> Result<Time<TAI>, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<TAI>>::into_time(raw))
    }
}

impl InfallibleFormatForScale<TAI> for GPS {
    #[inline]
    fn from_time(time: Time<TAI>) -> Second {
        time.raw_gps_seconds()
    }

    #[inline]
    fn into_time(raw: Second) -> Time<TAI> {
        Time::from_raw_gps_seconds(raw).expect("finite GPS seconds must decode")
    }
}

// ── From/Into between EncodedTime and Time ────────────────────────────────────

impl<S: Scale, F> From<EncodedTime<S, F>> for Time<S>
where
    F: InfallibleFormatForScale<S>,
{
    #[inline]
    fn from(value: EncodedTime<S, F>) -> Self {
        value.to_time()
    }
}

impl<S: Scale, F> From<Time<S>> for EncodedTime<S, F>
where
    F: InfallibleFormatForScale<S>,
{
    #[inline]
    fn from(value: Time<S>) -> Self {
        Self::from_time_infallible(value)
    }
}

// ── ConversionTarget impls for format markers ─────────────────────────────────

impl<S: CoordinateScale> ConversionTarget<S> for J2000s {
    type Output = EncodedTime<S, J2000s>;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        Ok(EncodedTime::from_time_infallible(src))
    }
}

impl<S: CoordinateScale> InfallibleConversionTarget<S> for J2000s {
    #[inline]
    fn convert(src: Time<S>) -> Self::Output {
        EncodedTime::from_time_infallible(src)
    }
}

impl<S: CoordinateScale> ConversionTarget<S> for JD {
    type Output = EncodedTime<S, JD>;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        Ok(EncodedTime::from_time_infallible(src))
    }
}

impl<S: CoordinateScale> InfallibleConversionTarget<S> for JD {
    #[inline]
    fn convert(src: Time<S>) -> Self::Output {
        EncodedTime::from_time_infallible(src)
    }
}

impl<S: CoordinateScale> ConversionTarget<S> for MJD {
    type Output = EncodedTime<S, MJD>;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        Ok(EncodedTime::from_time_infallible(src))
    }
}

impl<S: CoordinateScale> InfallibleConversionTarget<S> for MJD {
    #[inline]
    fn convert(src: Time<S>) -> Self::Output {
        EncodedTime::from_time_infallible(src)
    }
}

impl<S> ConversionTarget<S> for Unix
where
    S: crate::scale::Scale + InfallibleScaleConvert<UTC>,
{
    type Output = EncodedTime<UTC, Unix>;

    /// Snapshots the active time-data bundle at call time via
    /// [`TimeContext::new`]. For reproducible pipelines, prefer
    /// [`to_with::<Unix>(&ctx)`](crate::time::Time::to_with).
    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        let utc = src.to_scale::<UTC>();
        let raw = Unix::try_from_time(utc, &TimeContext::new())?;
        Ok(EncodedTime::new_unchecked(raw))
    }
}

impl ContextConversionTarget<UTC> for Unix {
    type Output = EncodedTime<UTC, Unix>;

    #[inline]
    fn convert_with(src: Time<UTC>, ctx: &TimeContext) -> Result<Self::Output, ConversionError> {
        let raw = Unix::try_from_time(src, ctx)?;
        Ok(EncodedTime::new_unchecked(raw))
    }
}

impl<S> ContextConversionTarget<S> for Unix
where
    S: crate::scale::Scale + crate::scale::conversion::ContextScaleConvert<UTC>,
{
    type Output = EncodedTime<UTC, Unix>;

    #[inline]
    fn convert_with(src: Time<S>, ctx: &TimeContext) -> Result<Self::Output, ConversionError> {
        let utc = src.to_scale_with::<UTC>(ctx)?;
        let raw = Unix::try_from_time(utc, ctx)?;
        Ok(EncodedTime::new_unchecked(raw))
    }
}

impl<S> ConversionTarget<S> for GPS
where
    S: crate::scale::Scale + InfallibleScaleConvert<TAI>,
{
    type Output = EncodedTime<TAI, GPS>;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        Ok(Self::convert(src))
    }
}

impl<S> InfallibleConversionTarget<S> for GPS
where
    S: crate::scale::Scale + InfallibleScaleConvert<TAI>,
{
    #[inline]
    fn convert(src: Time<S>) -> Self::Output {
        EncodedTime::from_time_infallible(src.to_scale::<TAI>())
    }
}

impl<S> ContextConversionTarget<S> for GPS
where
    S: crate::scale::Scale + crate::scale::conversion::ContextScaleConvert<TAI>,
{
    type Output = EncodedTime<TAI, GPS>;

    #[inline]
    fn convert_with(src: Time<S>, ctx: &TimeContext) -> Result<Self::Output, ConversionError> {
        let tai = src.to_scale_with::<TAI>(ctx)?;
        Ok(EncodedTime::from_time_infallible(tai))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::{TT, UTC};
    use qtty::Day;

    #[test]
    fn encoded_time_display_delegates_to_quantity() {
        let jd = JulianDate::<TT>::try_new(Day::new(2_451_545.123_456_789)).unwrap();

        assert_eq!(format!("{jd:.9}"), "2451545.123456789 d");
    }

    #[test]
    fn encoded_time_lower_exp_delegates_to_quantity() {
        use qtty::Second;
        let seconds = J2000Seconds::<TT>::try_new(Second::new(1_234.5)).unwrap();
        let formatted = format!("{seconds:.2e}");

        assert!(formatted.contains("e"));
        assert!(formatted.ends_with(" s"));
    }

    #[test]
    fn debug_includes_format_and_scale() {
        let jd = JulianDate::<TT>::try_new(Day::new(2_451_545.0)).unwrap();
        let dbg = format!("{jd:?}");
        assert!(dbg.contains("TT"), "debug should contain scale name");
        assert!(dbg.contains("JD"), "debug should contain format name");
    }

    /// Verifies that `EncodedTime<TT, JD>` and `EncodedTime<UTC, JD>` are
    /// statically distinct types that cannot be accidentally interchanged.
    ///
    /// The phantom scale parameter makes a Julian Date on TT and a Julian Date
    /// on UTC completely different types even though both hold a `Day` quantity.
    #[test]
    fn jd_on_tt_and_utc_are_distinct_types() {
        fn accept_tt(x: EncodedTime<TT, JD>) -> Day {
            x.raw()
        }
        fn accept_utc(x: EncodedTime<UTC, JD>) -> Day {
            x.raw()
        }

        let tt_jd = JulianDate::<TT>::try_new(Day::new(2_451_545.0)).unwrap();
        let utc_jd = JulianDate::<UTC>::try_new(Day::new(2_451_545.0)).unwrap();

        // Both are valid individually; the types enforce scale separation.
        let _ = accept_tt(tt_jd);
        let _ = accept_utc(utc_jd);
    }

    #[test]
    fn format_names_are_correct() {
        assert_eq!(JD::NAME, "JD");
        assert_eq!(MJD::NAME, "MJD");
        assert_eq!(J2000s::NAME, "J2000s");
        assert_eq!(Unix::NAME, "Unix");
        assert_eq!(GPS::NAME, "GPS");
    }
}
