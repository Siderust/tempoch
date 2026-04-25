// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion-target markers for the unified `Time::to::<T>()` API.
//!
//! Format-encoding targets (`JD`, `MJD`, `J2000s`, `Unix`, `GPS`) live in
//! [`crate::representation`]. This module provides the trait definitions and
//! scale-to-scale conversion impls.

use crate::context::TimeContext;
use crate::error::ConversionError;
use crate::scale::conversion::{ContextScaleConvert, InfallibleScaleConvert};
use crate::scale::{Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
use crate::sealed::Sealed;
use crate::time::Time;

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

/// Implements [`ConversionTarget`] for a scale pair that requires a
/// [`TimeContext`] (i.e. UT1 conversions), using a fresh default snapshot.
/// For reproducible pipelines, prefer [`ContextConversionTarget`] via
/// [`Time::to_with`](crate::time::Time::to_with).
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

#[cfg(test)]
mod tests {
    use crate::representation::{J2000s, JD, MJD, Unix, GPS};
    use crate::scale::{TAI, TT, UT1, UTC};
    use qtty::Second;

    #[test]
    fn scalar_targets_match_coordinate_helpers() {
        let tt = crate::time::Time::<TT>::from_raw_j2000_seconds(Second::new(12_345.678)).unwrap();

        assert_eq!(tt.to::<J2000s>().raw(), tt.raw_j2000_seconds());
        assert_eq!(tt.to::<JD>().raw(), crate::encoding::j2000_seconds_to_jd(tt.raw_j2000_seconds()));
        assert_eq!(tt.to::<MJD>().raw(), crate::encoding::j2000_seconds_to_mjd(tt.raw_j2000_seconds()));
    }

    #[test]
    fn unix_and_gps_targets_use_expected_axes() {
        let ctx = crate::context::TimeContext::new();
        let utc = crate::time::Time::<UTC>::from_raw_unix_seconds_with(
            Second::new(946_728_000.0),
            &ctx,
        )
        .unwrap();
        let unix = utc.try_to::<Unix>().unwrap();
        assert!((unix.raw() - utc.raw_unix_seconds_with(&ctx).unwrap()).abs() < Second::new(1e-12));

        let tai = utc.to::<TAI>();
        let gps = tai.to::<GPS>();
        assert!((gps.raw() - tai.raw_gps_seconds()).abs() < Second::new(1e-12));

        let gps_from_tt = crate::time::Time::<TT>::from_raw_j2000_seconds(Second::new(0.0))
            .unwrap()
            .to::<GPS>();
        assert!(gps_from_tt.raw().is_finite());
    }

    #[test]
    fn default_context_ut1_routes_are_reachable() {
        let tt = crate::time::Time::<TT>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
        let ut1 = tt.try_to::<UT1>().unwrap();
        let tt_back = ut1.try_to::<TT>().unwrap();
        let utc_back = ut1.try_to::<UTC>().unwrap();

        assert!(tt_back.raw_j2000_seconds().is_finite());
        assert!(utc_back.raw_j2000_seconds().is_finite());
    }

    #[test]
    fn context_targets_support_ut1_sources() {
        let tt = crate::time::Time::<TT>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
        let ctx = crate::context::TimeContext::new();
        let ut1 = tt.to_with::<UT1>(&ctx).unwrap();

        let unix = ut1.to_with::<Unix>(&ctx).unwrap();
        let gps = ut1.to_with::<GPS>(&ctx).unwrap();

        assert!(unix.raw().is_finite());
        assert!(gps.raw().is_finite());
    }
}
