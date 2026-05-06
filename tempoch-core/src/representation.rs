// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed encoded representations of [`crate::Time`].

use core::fmt;
use core::marker::PhantomData;

use crate::context::TimeContext;
use crate::encoding::{
    j2000_seconds_to_jd, j2000_seconds_to_mjd, jd_to_j2000_seconds, mjd_to_j2000_seconds,
};
use crate::error::ConversionError;
use crate::scale::conversion::InfallibleScaleConvert;
use crate::scale::{CoordinateScale, Scale, TAI, TDB, TT, UTC};
use crate::sealed::Sealed;
use crate::target::{ContextConversionTarget, ConversionTarget, InfallibleConversionTarget};
use crate::time::Time;
use qtty::{Day, Quantity, Second, Unit};

/// Marker trait for external time encodings such as JD or Unix time.
#[allow(private_bounds)]
pub trait TimeRepresentation: Sealed + Copy + Clone + fmt::Debug + 'static {
    /// Quantity unit used by this representation.
    type Unit: Unit;

    /// Human-readable representation name.
    const NAME: &'static str;
}

/// Representation witness for scale `S`.
#[allow(private_bounds)]
pub trait RepresentationForScale<S: Scale>: TimeRepresentation + Sealed {
    fn try_from_time(
        time: Time<S>,
        ctx: &TimeContext,
    ) -> Result<Quantity<Self::Unit>, ConversionError>;
    fn try_into_time(
        raw: Quantity<Self::Unit>,
        ctx: &TimeContext,
    ) -> Result<Time<S>, ConversionError>;
}

/// Representation witness for scale `S` with context-free round-trips.
#[allow(private_bounds)]
pub trait InfallibleRepresentationForScale<S: Scale>: RepresentationForScale<S> + Sealed {
    fn from_time(time: Time<S>) -> Quantity<Self::Unit>;
    fn into_time(raw: Quantity<Self::Unit>) -> Time<S>;
}

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

impl TimeRepresentation for J2000s {
    type Unit = qtty::unit::Second;
    const NAME: &'static str = "J2000s";
}

impl TimeRepresentation for JD {
    type Unit = qtty::unit::Day;
    const NAME: &'static str = "Julian Day";
}

impl TimeRepresentation for MJD {
    type Unit = qtty::unit::Day;
    const NAME: &'static str = "Modified Julian Day";
}

impl TimeRepresentation for Unix {
    type Unit = qtty::unit::Second;
    const NAME: &'static str = "Unix";
}

impl TimeRepresentation for GPS {
    type Unit = qtty::unit::Second;
    const NAME: &'static str = "GPS";
}

/// A typed external representation of a [`Time<S>`] instant.
pub struct EncodedTime<S: Scale, R: TimeRepresentation> {
    raw: Quantity<R::Unit>,
    _marker: PhantomData<fn() -> S>,
}

impl<S: Scale, R: TimeRepresentation> Copy for EncodedTime<S, R> {}

impl<S: Scale, R: TimeRepresentation> Clone for EncodedTime<S, R> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: Scale, R: TimeRepresentation> fmt::Debug for EncodedTime<S, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncodedTime")
            .field("scale", &S::NAME)
            .field("representation", &R::NAME)
            .field("raw", &self.raw)
            .finish()
    }
}

impl<S: Scale, R: TimeRepresentation> fmt::Display for EncodedTime<S, R>
where
    qtty::Quantity<R::Unit>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", R::NAME)?;
        fmt::Display::fmt(&self.raw, f)
    }
}

impl<S: Scale, R: TimeRepresentation> fmt::LowerExp for EncodedTime<S, R>
where
    qtty::Quantity<R::Unit>: fmt::LowerExp,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerExp::fmt(&self.raw, f)
    }
}

impl<S: Scale, R: TimeRepresentation> fmt::UpperExp for EncodedTime<S, R>
where
    qtty::Quantity<R::Unit>: fmt::UpperExp,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperExp::fmt(&self.raw, f)
    }
}

impl<S: Scale, R: TimeRepresentation> PartialEq for EncodedTime<S, R> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<S: Scale, R: TimeRepresentation> PartialOrd for EncodedTime<S, R> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.raw.partial_cmp(&other.raw)
    }
}

impl<S: Scale, R: TimeRepresentation> EncodedTime<S, R> {
    /// Construct from a raw quantity, bypassing the finite check.
    ///
    /// Passing a non-finite value yields an instant whose behaviour is
    /// unspecified. Prefer [`Self::try_new`] for user-supplied data; use this
    /// only when the value is known to be finite (e.g. compile-time constants).
    #[inline]
    pub const fn new_unchecked(raw: Quantity<R::Unit>) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }

    /// Return the underlying typed quantity.
    #[inline]
    pub const fn raw(self) -> Quantity<R::Unit> {
        self.raw
    }

    /// Alias for [`Self::raw`].
    #[inline]
    pub const fn quantity(self) -> Quantity<R::Unit> {
        self.raw
    }
}

impl<S: Scale, R> EncodedTime<S, R>
where
    R: RepresentationForScale<S>,
{
    /// Construct a typed encoded instant from its raw quantity.
    #[inline]
    pub fn try_new(raw: Quantity<R::Unit>) -> Result<Self, ConversionError> {
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
        R::try_into_time(self.raw, &TimeContext::new())
    }

    /// Convert this encoded instant to the canonical [`Time<S>`] model using an explicit context.
    #[inline]
    pub fn to_time_with(self, ctx: &TimeContext) -> Result<Time<S>, ConversionError> {
        R::try_into_time(self.raw, ctx)
    }
}

impl<S: Scale, R> EncodedTime<S, R>
where
    R: InfallibleRepresentationForScale<S>,
{
    #[inline]
    pub(crate) fn from_time_infallible(time: Time<S>) -> Self {
        Self::new_unchecked(R::from_time(time))
    }

    /// Infallible conversion to the canonical [`Time<S>`] model.
    #[inline]
    pub fn to_time(self) -> Time<S> {
        R::into_time(self.raw)
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

impl<S: Scale, R> EncodedTime<S, R>
where
    R: RepresentationForScale<S>,
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

/// J2000.0 epoch as a TT-scale Julian Date (JD(TT) = 2 451 545.0).
pub const J2000_TT: JulianDate<TT> = EncodedTime::<TT, JD>::new_unchecked(Day::new(2_451_545.0));

// ── Inherent helpers for Day-based encoded times (JD and MJD) ────────────────

impl<S: Scale, R> EncodedTime<S, R>
where
    R: TimeRepresentation<Unit = qtty::unit::Day>,
{
    /// Earlier of `self` and `other`.
    ///
    /// Equivalent to `if self <= other { self } else { other }`.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        if self.raw.value() <= other.raw.value() {
            self
        } else {
            other
        }
    }

    /// Later of `self` and `other`.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        if self.raw.value() >= other.raw.value() {
            self
        } else {
            other
        }
    }

    /// Midpoint between `self` and `other`.
    #[inline]
    pub fn mean(self, other: Self) -> Self {
        Self::new_unchecked(Day::new((self.raw.value() + other.raw.value()) * 0.5))
    }
}

// ── JulianDate<S> inherent helpers ───────────────────────────────────────────

/// Length of a Julian year in days (exactly 365.25 d).
pub const JULIAN_YEAR_DAYS: Day = Day::new(365.25);

impl<S: CoordinateScale> JulianDate<S> {
    /// Construct from a raw Julian Day value without validation.
    ///
    /// Prefer [`Self::try_new`] for untrusted input.
    #[inline]
    pub fn new(jd: f64) -> Self {
        Self::new_unchecked(Day::new(jd))
    }

    /// Raw Julian Day value as `f64` (days since noon 1 January 4713 BC JD).
    #[inline]
    pub fn jd_value(self) -> f64 {
        self.raw().value()
    }

    /// Julian centuries since J2000.0: `T = (JD − 2 451 545.0) / 36 525`.
    #[inline]
    pub fn julian_centuries(self) -> f64 {
        (self.raw().value() - crate::constats::J2000_JD_TT.value())
            / crate::constats::DAYS_PER_JC.value()
    }

    /// Julian millennia since J2000.0: `T = (JD − 2 451 545.0) / 365 250`.
    #[inline]
    pub fn julian_millennias(self) -> f64 {
        (self.raw().value() - crate::constats::J2000_JD_TT.value())
            / (crate::constats::DAYS_PER_JC.value() * 10.0)
    }

    /// Length of a Julian year in days (365.25 d).
    pub const JULIAN_YEAR: Day = Day::new(365.25);

    /// Length of a Julian century in days (36 525 d).
    pub const JULIAN_CENTURY: Day = Day::new(36_525.0);
}

impl JulianDate<TT> {
    /// J2000.0 epoch as a TT-scale Julian Date (`JD(TT) = 2 451 545.0`).
    pub const J2000: Self = J2000_TT;

    /// Convert this TT Julian Date to the TDB scale.
    ///
    /// Uses the Fairhead-Bretagnon periodic correction stored in the scale
    /// conversion layer. The result is on the TDB coordinate time axis, still
    /// expressed as a Julian Date.
    #[inline]
    pub fn tt_to_tdb(self) -> JulianDate<TDB> {
        JulianDate::<TDB>::from_time_infallible(self.to_time().to_scale::<TDB>())
    }

    /// Build a TT Julian Date from a `chrono::DateTime<Utc>`.
    ///
    /// Converts UTC → TAI → TT internally.  Panics if the UTC time data is
    /// unavailable for the supplied instant; use
    /// [`Time::<UTC>::try_from_chrono`] for a fallible path.
    #[inline]
    pub fn from_chrono(dt: chrono::DateTime<chrono::Utc>) -> Self {
        let utc = Time::<UTC>::from_chrono(dt);
        Self::from_time_infallible(utc.to_scale::<TT>())
    }

    /// Convert this TT Julian Date to a `chrono::DateTime<Utc>`.
    ///
    /// Returns `None` if the value is outside the supported UTC range.
    #[inline]
    pub fn to_chrono(self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.to_time().to_scale::<UTC>().to_chrono()
    }
}

// ── Cross-representation From conversions (JD ↔ MJD for CoordinateScales) ───

impl<S: CoordinateScale> From<ModifiedJulianDate<S>> for JulianDate<S> {
    /// Convert a Modified Julian Date to a Julian Date on the same scale.
    #[inline]
    fn from(mjd: ModifiedJulianDate<S>) -> Self {
        mjd.to::<JD>()
    }
}

impl<S: CoordinateScale> From<JulianDate<S>> for ModifiedJulianDate<S> {
    /// Convert a Julian Date to a Modified Julian Date on the same scale.
    #[inline]
    fn from(jd: JulianDate<S>) -> Self {
        jd.to::<MJD>()
    }
}

// ── ModifiedJulianDate<S> inherent helpers ────────────────────────────────────

impl<S: CoordinateScale> ModifiedJulianDate<S> {
    /// Construct from a raw Modified Julian Day value without validation.
    ///
    /// Prefer [`Self::try_new`] for untrusted input.
    #[inline]
    pub fn new(mjd: f64) -> Self {
        Self::new_unchecked(Day::new(mjd))
    }

    /// Raw Modified Julian Day value as `f64` (days since midnight 17 November 1858).
    #[inline]
    pub fn mjd_value(self) -> f64 {
        self.raw().value()
    }
}

impl ModifiedJulianDate<TT> {
    /// Build a TT Modified Julian Date from a `chrono::DateTime<Utc>`.
    ///
    /// Converts UTC → TAI → TT internally.  Panics if UTC time data is
    /// unavailable; use [`Time::<UTC>::try_from_chrono`] for a fallible path.
    #[inline]
    pub fn from_chrono(dt: chrono::DateTime<chrono::Utc>) -> Self {
        let utc = Time::<UTC>::from_chrono(dt);
        Self::from_time_infallible(utc.to_scale::<TT>())
    }

    /// Convert this TT Modified Julian Date to a `chrono::DateTime<Utc>`.
    ///
    /// Returns `None` if the value is outside the supported UTC range.
    #[inline]
    pub fn to_chrono(self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.to_time().to_scale::<UTC>().to_chrono()
    }
}

macro_rules! coordinate_representation {
    ($repr:ty, $quantity:ty, $from_time:expr, $to_time:expr) => {
        impl<S: CoordinateScale> RepresentationForScale<S> for $repr {
            #[inline]
            fn try_from_time(
                time: Time<S>,
                _ctx: &TimeContext,
            ) -> Result<$quantity, ConversionError> {
                Ok(<Self as InfallibleRepresentationForScale<S>>::from_time(
                    time,
                ))
            }

            #[inline]
            fn try_into_time(
                raw: $quantity,
                _ctx: &TimeContext,
            ) -> Result<Time<S>, ConversionError> {
                Ok(<Self as InfallibleRepresentationForScale<S>>::into_time(
                    raw,
                ))
            }
        }

        impl<S: CoordinateScale> InfallibleRepresentationForScale<S> for $repr {
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

coordinate_representation!(
    J2000s,
    Second,
    |time: Time<_>| time.raw_j2000_seconds(),
    |raw: Second| Time::from_raw_j2000_seconds(raw).expect("finite J2000 seconds must decode")
);
coordinate_representation!(
    JD,
    Day,
    |time: Time<_>| j2000_seconds_to_jd(time.raw_j2000_seconds()),
    |raw: Day| Time::from_raw_j2000_seconds(jd_to_j2000_seconds(raw))
        .expect("finite Julian date must decode")
);
coordinate_representation!(
    MJD,
    Day,
    |time: Time<_>| j2000_seconds_to_mjd(time.raw_j2000_seconds()),
    |raw: Day| Time::from_raw_j2000_seconds(mjd_to_j2000_seconds(raw))
        .expect("finite Modified Julian date must decode")
);

impl RepresentationForScale<UTC> for Unix {
    #[inline]
    fn try_from_time(time: Time<UTC>, ctx: &TimeContext) -> Result<Second, ConversionError> {
        time.raw_unix_seconds_with(ctx)
    }

    #[inline]
    fn try_into_time(raw: Second, ctx: &TimeContext) -> Result<Time<UTC>, ConversionError> {
        Time::from_raw_unix_seconds_with(raw, ctx)
    }
}

impl RepresentationForScale<TAI> for GPS {
    #[inline]
    fn try_from_time(time: Time<TAI>, _ctx: &TimeContext) -> Result<Second, ConversionError> {
        Ok(<Self as InfallibleRepresentationForScale<TAI>>::from_time(
            time,
        ))
    }

    #[inline]
    fn try_into_time(raw: Second, _ctx: &TimeContext) -> Result<Time<TAI>, ConversionError> {
        Ok(<Self as InfallibleRepresentationForScale<TAI>>::into_time(
            raw,
        ))
    }
}

impl InfallibleRepresentationForScale<TAI> for GPS {
    #[inline]
    fn from_time(time: Time<TAI>) -> Second {
        time.raw_gps_seconds()
    }

    #[inline]
    fn into_time(raw: Second) -> Time<TAI> {
        Time::from_raw_gps_seconds(raw).expect("finite GPS seconds must decode")
    }
}

// ── Arithmetic on EncodedTime ────────────────────────────────────────────────
//
// For JD- and MJD-based representations (both use `qtty::unit::Day`), it is
// natural to shift an instant by a number of days and to compute the signed
// duration between two instants.

impl<S: Scale, R> core::ops::Add<Day> for EncodedTime<S, R>
where
    R: TimeRepresentation<Unit = qtty::unit::Day>,
{
    type Output = Self;

    #[inline]
    fn add(self, rhs: Day) -> Self {
        Self::new_unchecked(Day::new(self.raw.value() + rhs.value()))
    }
}

impl<S: Scale, R> core::ops::Sub<Day> for EncodedTime<S, R>
where
    R: TimeRepresentation<Unit = qtty::unit::Day>,
{
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Day) -> Self {
        Self::new_unchecked(Day::new(self.raw.value() - rhs.value()))
    }
}

impl<S: Scale, R> core::ops::AddAssign<Day> for EncodedTime<S, R>
where
    R: TimeRepresentation<Unit = qtty::unit::Day>,
{
    #[inline]
    fn add_assign(&mut self, rhs: Day) {
        *self = *self + rhs;
    }
}

impl<S: Scale, R> core::ops::SubAssign<Day> for EncodedTime<S, R>
where
    R: TimeRepresentation<Unit = qtty::unit::Day>,
{
    #[inline]
    fn sub_assign(&mut self, rhs: Day) {
        *self = *self - rhs;
    }
}

/// `b - a` returns the signed offset in days: positive when `b` is later.
impl<S: Scale, R> core::ops::Sub for EncodedTime<S, R>
where
    R: TimeRepresentation<Unit = qtty::unit::Day>,
{
    type Output = Day;

    #[inline]
    fn sub(self, rhs: Self) -> Day {
        Day::new(self.raw.value() - rhs.raw.value())
    }
}

impl<S: Scale, R> From<EncodedTime<S, R>> for Time<S>
where
    R: InfallibleRepresentationForScale<S>,
{
    #[inline]
    fn from(value: EncodedTime<S, R>) -> Self {
        value.to_time()
    }
}

impl<S: Scale, R> From<Time<S>> for EncodedTime<S, R>
where
    R: InfallibleRepresentationForScale<S>,
{
    #[inline]
    fn from(value: Time<S>) -> Self {
        Self::from_time_infallible(value)
    }
}

// ── ConversionTarget impls for format markers ────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::TT;

    #[test]
    fn encoded_time_display_delegates_to_quantity() {
        let jd = JulianDate::<TT>::try_new(Day::new(2_451_545.123_456_789)).unwrap();

        assert_eq!(format!("{jd:.9}"), "Julian Day 2451545.123456789 d");
    }

    #[test]
    fn encoded_time_lower_exp_delegates_to_quantity() {
        let seconds = J2000Seconds::<TT>::try_new(Second::new(1_234.5)).unwrap();
        let formatted = format!("{seconds:.2e}");

        assert!(formatted.contains("e"));
        assert!(formatted.ends_with(" s"));
    }
}
