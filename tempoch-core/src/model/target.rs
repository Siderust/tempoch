// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion-target markers for the unified `Time::to::<T>()` API.
//!
//! Format markers (`JD`, `MJD`, `J2000s`, `Unix`, `GPS`) and scale markers
//! (`TT`, `TAI`, …) implement these traits. The source instant may carry any
//! format phantom `SrcF`; storage is always the compensated J2000-second pair.

use crate::earth::context::TimeContext;
use crate::format::markers::{J2000s, Unix, GPS, JD, MJD};
use crate::format::TimeFormat;
use crate::foundation::error::ConversionError;
use crate::foundation::sealed::Sealed;
use crate::model::scale::conversion::{ContextScaleConvert, InfallibleScaleConvert};
use crate::model::scale::{
    CoordinateScale, Scale, BDT, ET, GPST, GST, QZSST, TAI, TCB, TCG, TDB, TT, UT1, UTC,
};
use crate::model::time::Time;

/// Unified conversion target for `Time<S, F>::try_to::<T>()`.
#[allow(private_bounds)]
pub trait ConversionTarget<S: Scale, SrcF: TimeFormat = J2000s>: Sealed {
    type Output;

    fn try_convert(src: Time<S, SrcF>) -> Result<Self::Output, ConversionError>;
}

/// Unified infallible conversion target for `Time<S, F>::to::<T>()`.
#[allow(private_bounds)]
pub trait InfallibleConversionTarget<S: Scale, SrcF: TimeFormat = J2000s>:
    ConversionTarget<S, SrcF> + Sealed
{
    fn convert(src: Time<S, SrcF>) -> Self::Output;
}

/// Unified context-backed conversion target for `Time<S, F>::to_with::<T>(&ctx)`.
#[allow(private_bounds)]
pub trait ContextConversionTarget<S: Scale, SrcF: TimeFormat = J2000s>: Sealed {
    type Output;

    fn convert_with(src: Time<S, SrcF>, ctx: &TimeContext)
        -> Result<Self::Output, ConversionError>;
}

impl<S: CoordinateScale, SrcF: TimeFormat> ConversionTarget<S, SrcF> for J2000s {
    type Output = Time<S, J2000s>;

    #[inline]
    fn try_convert(src: Time<S, SrcF>) -> Result<Self::Output, ConversionError> {
        Ok(src.reinterpret())
    }
}

impl<S: CoordinateScale, SrcF: TimeFormat> InfallibleConversionTarget<S, SrcF> for J2000s {
    #[inline]
    fn convert(src: Time<S, SrcF>) -> Self::Output {
        src.reinterpret()
    }
}

impl<S: CoordinateScale, SrcF: TimeFormat> ConversionTarget<S, SrcF> for JD {
    type Output = Time<S, JD>;

    #[inline]
    fn try_convert(src: Time<S, SrcF>) -> Result<Self::Output, ConversionError> {
        Ok(<JD as InfallibleConversionTarget<S, SrcF>>::convert(src))
    }
}

impl<S: CoordinateScale, SrcF: TimeFormat> InfallibleConversionTarget<S, SrcF> for JD {
    #[inline]
    fn convert(src: Time<S, SrcF>) -> Self::Output {
        src.reinterpret()
    }
}

impl<S: CoordinateScale, SrcF: TimeFormat> ConversionTarget<S, SrcF> for MJD {
    type Output = Time<S, MJD>;

    #[inline]
    fn try_convert(src: Time<S, SrcF>) -> Result<Self::Output, ConversionError> {
        Ok(<MJD as InfallibleConversionTarget<S, SrcF>>::convert(src))
    }
}

impl<S: CoordinateScale, SrcF: TimeFormat> InfallibleConversionTarget<S, SrcF> for MJD {
    #[inline]
    fn convert(src: Time<S, SrcF>) -> Self::Output {
        src.reinterpret()
    }
}

impl<S1: Scale + InfallibleScaleConvert<S2>, S2: Scale, SrcF: TimeFormat> ConversionTarget<S1, SrcF>
    for S2
{
    type Output = Time<S2, SrcF>;

    #[inline]
    fn try_convert(src: Time<S1, SrcF>) -> Result<Self::Output, ConversionError> {
        Ok(<S2 as InfallibleConversionTarget<S1, SrcF>>::convert(src))
    }
}

impl<S1: Scale + InfallibleScaleConvert<S2>, S2: Scale, SrcF: TimeFormat>
    InfallibleConversionTarget<S1, SrcF> for S2
{
    #[inline]
    fn convert(src: Time<S1, SrcF>) -> Self::Output {
        src.to_scale()
    }
}

impl<S1: Scale + ContextScaleConvert<S2>, S2: Scale, SrcF: TimeFormat>
    ContextConversionTarget<S1, SrcF> for S2
{
    type Output = Time<S2, SrcF>;

    #[inline]
    fn convert_with(
        src: Time<S1, SrcF>,
        ctx: &TimeContext,
    ) -> Result<Self::Output, ConversionError> {
        src.to_scale_with(ctx)
    }
}

/// Implements [`ConversionTarget`] for a scale pair that requires a
/// [`TimeContext`] (i.e. UT1 conversions), using a fresh default snapshot.
macro_rules! default_context_scale_target {
    ($src:ty => $dst:ty) => {
        impl<SrcF: TimeFormat> ConversionTarget<$src, SrcF> for $dst {
            type Output = Time<$dst, SrcF>;

            #[inline]
            fn try_convert(src: Time<$src, SrcF>) -> Result<Self::Output, ConversionError> {
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
default_context_scale_target!(ET => UT1);
default_context_scale_target!(GPST => UT1);
default_context_scale_target!(GST => UT1);
default_context_scale_target!(QZSST => UT1);
default_context_scale_target!(BDT => UT1);
default_context_scale_target!(UT1 => TT);
default_context_scale_target!(UT1 => TAI);
default_context_scale_target!(UT1 => TDB);
default_context_scale_target!(UT1 => TCG);
default_context_scale_target!(UT1 => TCB);
default_context_scale_target!(UT1 => UTC);
default_context_scale_target!(UT1 => ET);
default_context_scale_target!(UT1 => GPST);
default_context_scale_target!(UT1 => GST);
default_context_scale_target!(UT1 => QZSST);
default_context_scale_target!(UT1 => BDT);

impl<S: Scale + InfallibleScaleConvert<UTC>, SrcF: TimeFormat> ConversionTarget<S, SrcF> for Unix {
    type Output = Time<UTC, Unix>;

    #[inline]
    fn try_convert(src: Time<S, SrcF>) -> Result<Self::Output, ConversionError> {
        let ctx = TimeContext::new();
        let utc = src.to_scale::<UTC>();
        utc.to_j2000s().raw_unix_seconds_with(&ctx)?;
        Ok(utc.reinterpret())
    }
}

impl<S: Scale + ContextScaleConvert<UTC>, SrcF: TimeFormat> ContextConversionTarget<S, SrcF>
    for Unix
{
    type Output = Time<UTC, Unix>;

    #[inline]
    fn convert_with(
        src: Time<S, SrcF>,
        ctx: &TimeContext,
    ) -> Result<Self::Output, ConversionError> {
        let utc = src.to_scale_with::<UTC>(ctx)?;
        utc.to_j2000s().raw_unix_seconds_with(ctx)?;
        Ok(utc.reinterpret())
    }
}

impl<S: Scale + InfallibleScaleConvert<TAI>, SrcF: TimeFormat> ConversionTarget<S, SrcF> for GPS {
    type Output = Time<TAI, GPS>;

    #[inline]
    fn try_convert(src: Time<S, SrcF>) -> Result<Self::Output, ConversionError> {
        Ok(<GPS as InfallibleConversionTarget<S, SrcF>>::convert(src))
    }
}

impl<S: Scale + InfallibleScaleConvert<TAI>, SrcF: TimeFormat> InfallibleConversionTarget<S, SrcF>
    for GPS
{
    #[inline]
    fn convert(src: Time<S, SrcF>) -> Self::Output {
        src.to_scale::<TAI>().reinterpret()
    }
}

impl<S: Scale + ContextScaleConvert<TAI>, SrcF: TimeFormat> ContextConversionTarget<S, SrcF>
    for GPS
{
    type Output = Time<TAI, GPS>;

    #[inline]
    fn convert_with(
        src: Time<S, SrcF>,
        ctx: &TimeContext,
    ) -> Result<Self::Output, ConversionError> {
        Ok(src.to_scale_with::<TAI>(ctx)?.reinterpret())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::{J2000s, Unix, GPS, JD, MJD};
    use crate::model::scale::{TAI, TT, UT1, UTC};
    use qtty::Second;

    #[test]
    fn scalar_targets_match_coordinate_helpers() {
        let tt = crate::model::time::Time::<TT>::from_raw_j2000_seconds(Second::new(12_345.678))
            .unwrap();

        assert_eq!(tt.to::<J2000s>().raw(), tt.raw_j2000_seconds());
        assert_eq!(
            tt.to::<JD>().raw(),
            crate::encoding::j2000_seconds_to_day::<JD>(tt.raw_j2000_seconds())
        );
        assert_eq!(
            tt.to::<MJD>().raw(),
            crate::encoding::j2000_seconds_to_day::<MJD>(tt.raw_j2000_seconds())
        );
    }

    #[test]
    fn unix_and_gps_targets_use_expected_axes() {
        let ctx = crate::earth::context::TimeContext::new();
        let utc = crate::model::time::Time::<UTC>::from_raw_unix_seconds_with(
            Second::new(946_728_000.0),
            &ctx,
        )
        .unwrap();
        let unix = utc.try_to::<Unix>().unwrap();
        let unix_sec = unix.try_raw_with(&ctx).unwrap();
        assert!(
            (unix_sec - utc.to_j2000s().raw_unix_seconds_with(&ctx).unwrap()).abs()
                < Second::new(1e-12)
        );

        let tai = utc.to::<TAI>();
        let gps = tai.to::<GPS>();
        assert!((gps.raw() - tai.to_j2000s().raw_gps_seconds()).abs() < Second::new(1e-12));

        let gps_from_tt = crate::model::time::Time::<TT>::from_raw_j2000_seconds(Second::new(0.0))
            .unwrap()
            .to::<GPS>();
        assert!(gps_from_tt.raw().is_finite());
    }

    #[test]
    fn default_context_ut1_routes_are_reachable() {
        let tt = crate::model::time::Time::<TT>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
        let ut1 = tt.try_to::<UT1>().unwrap();
        let tt_back = ut1.try_to::<TT>().unwrap();
        let utc_back = ut1.try_to::<UTC>().unwrap();

        assert!(tt_back.raw_j2000_seconds().is_finite());
        assert!(utc_back.raw_j2000_seconds().is_finite());
    }

    #[test]
    fn context_targets_support_ut1_sources() {
        let tt = crate::model::time::Time::<TT>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
        let ctx = crate::earth::context::TimeContext::new();
        let ut1 = tt.to_with::<UT1>(&ctx).unwrap();

        let unix_sec = ut1
            .to_with::<Unix>(&ctx)
            .unwrap()
            .try_raw_with(&ctx)
            .unwrap();
        let gps = ut1.to_with::<GPS>(&ctx).unwrap();

        assert!(unix_sec.is_finite());
        assert!(gps.raw().is_finite());
    }
}
