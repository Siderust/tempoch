// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion-target markers for the unified `Time::to::<T>()` API.
//!
//! These tags are no longer storage parameters. They are request markers for
//! views and transport encodings over a canonical `Time<S>` value.

use crate::context::TimeContext;
use crate::error::ConversionError;
use crate::scale::conversion::{ContextScaleConvert, InfallibleScaleConvert};
use crate::scale::{CoordinateScale, Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
use crate::sealed::Sealed;
use crate::time::Time;
use qtty::{Day, Second};

/// Unified conversion target for `Time<S>::try_to::<T>()`.
#[allow(private_bounds)]
pub trait ConversionTarget<S: Scale>: Sealed {
    type Output;

    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError>;
}

/// Unified infallible conversion target for `Time<S>::to::<T>()`.
#[allow(private_bounds)]
pub trait InfallibleConversionTarget<S: Scale>: ConversionTarget<S> + Sealed {
    fn convert(src: Time<S>) -> Self::Output;
}

/// Unified context-backed conversion target for `Time<S>::to_with::<T>(&ctx)`.
#[allow(private_bounds)]
pub trait ContextConversionTarget<S: Scale>: Sealed {
    type Output;

    fn convert_with(src: Time<S>, ctx: &TimeContext) -> Result<Self::Output, ConversionError>;
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
pub struct UnixSecs;

/// GPS seconds since the GPS epoch on the TAI/GPS continuous axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GpsSecs;

impl Sealed for J2000s {}
impl Sealed for JD {}
impl Sealed for MJD {}
impl Sealed for UnixSecs {}
impl Sealed for GpsSecs {}

impl<S1, S2> ConversionTarget<S1> for S2
where
    S1: Scale + InfallibleScaleConvert<S2>,
    S2: Scale,
{
    type Output = Time<S2>;

    #[inline]
    fn try_convert(src: Time<S1>) -> Result<Self::Output, ConversionError> {
        Ok(Self::convert(src))
    }
}

impl<S1, S2> InfallibleConversionTarget<S1> for S2
where
    S1: Scale + InfallibleScaleConvert<S2>,
    S2: Scale,
{
    #[inline]
    fn convert(src: Time<S1>) -> Self::Output {
        src.to_scale()
    }
}

impl<S1, S2> ContextConversionTarget<S1> for S2
where
    S1: Scale + ContextScaleConvert<S2>,
    S2: Scale,
{
    type Output = Time<S2>;

    #[inline]
    fn convert_with(src: Time<S1>, ctx: &TimeContext) -> Result<Self::Output, ConversionError> {
        src.to_scale_with(ctx)
    }
}

impl<S: CoordinateScale> ConversionTarget<S> for J2000s {
    type Output = Second;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        Ok(Self::convert(src))
    }
}

impl<S: CoordinateScale> InfallibleConversionTarget<S> for J2000s {
    #[inline]
    fn convert(src: Time<S>) -> Self::Output {
        src.j2000_seconds()
    }
}

impl<S: CoordinateScale> ConversionTarget<S> for JD {
    type Output = Day;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        Ok(Self::convert(src))
    }
}

impl<S: CoordinateScale> InfallibleConversionTarget<S> for JD {
    #[inline]
    fn convert(src: Time<S>) -> Self::Output {
        src.julian_days()
    }
}

impl<S: CoordinateScale> ConversionTarget<S> for MJD {
    type Output = Day;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        Ok(Self::convert(src))
    }
}

impl<S: CoordinateScale> InfallibleConversionTarget<S> for MJD {
    #[inline]
    fn convert(src: Time<S>) -> Self::Output {
        src.modified_julian_days()
    }
}

impl<S> ConversionTarget<S> for UnixSecs
where
    S: Scale + InfallibleScaleConvert<UTC>,
{
    type Output = Second;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        src.to_scale::<UTC>().unix_seconds()
    }
}

impl<S> ContextConversionTarget<S> for UnixSecs
where
    S: Scale + ContextScaleConvert<UTC>,
{
    type Output = Second;

    #[inline]
    fn convert_with(src: Time<S>, ctx: &TimeContext) -> Result<Self::Output, ConversionError> {
        src.to_scale_with::<UTC>(ctx)?.unix_seconds_with(ctx)
    }
}

impl<S> ConversionTarget<S> for GpsSecs
where
    S: Scale + InfallibleScaleConvert<TAI>,
{
    type Output = Second;

    #[inline]
    fn try_convert(src: Time<S>) -> Result<Self::Output, ConversionError> {
        Ok(Self::convert(src))
    }
}

impl<S> InfallibleConversionTarget<S> for GpsSecs
where
    S: Scale + InfallibleScaleConvert<TAI>,
{
    #[inline]
    fn convert(src: Time<S>) -> Self::Output {
        src.to_scale::<TAI>().gps_seconds()
    }
}

impl<S> ContextConversionTarget<S> for GpsSecs
where
    S: Scale + ContextScaleConvert<TAI>,
{
    type Output = Second;

    #[inline]
    fn convert_with(src: Time<S>, ctx: &TimeContext) -> Result<Self::Output, ConversionError> {
        Ok(src.to_scale_with::<TAI>(ctx)?.gps_seconds())
    }
}

macro_rules! default_context_scale_target {
    ($src:ty => $dst:ty) => {
        impl ConversionTarget<$src> for $dst {
            type Output = Time<$dst>;

            #[inline]
            fn try_convert(src: Time<$src>) -> Result<Self::Output, ConversionError> {
                src.to_scale_with::<$dst>(&TimeContext::new())
            }
        }
    };
}

default_context_scale_target!(TT => UT1);
default_context_scale_target!(TAI => UT1);
default_context_scale_target!(TDB => UT1);
default_context_scale_target!(TCG => UT1);
default_context_scale_target!(TCB => UT1);
default_context_scale_target!(UTC => UT1);
default_context_scale_target!(UT1 => TT);
default_context_scale_target!(UT1 => TAI);
default_context_scale_target!(UT1 => TDB);
default_context_scale_target!(UT1 => TCG);
default_context_scale_target!(UT1 => TCB);
default_context_scale_target!(UT1 => UTC);
