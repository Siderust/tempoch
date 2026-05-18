// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! `chrono` convenience impls for TT-encoded instants whose format round-trips on TT
//! (e.g. [`JD`], [`MJD`], [`J2000s`] — not [`GPS`], which encodes TAI).

use crate::earth::context::TimeContext;
use crate::format::TimeFormat;
use crate::foundation::error::ConversionError;
use crate::model::scale::{TT, UTC};
use crate::model::target::InfallibleConversionTarget;
use crate::model::time::Time;
use crate::InfallibleFormatForScale;
use chrono::{DateTime, Utc};

impl<F: TimeFormat> Time<TT, F>
where
    F: InfallibleFormatForScale<TT> + InfallibleConversionTarget<TT, Output = Time<TT, F>>,
{
    /// Build a TT [`Time`] in format `F` from a UTC `chrono` timestamp.
    #[inline]
    pub fn try_from_chrono(dt: DateTime<Utc>) -> Result<Self, ConversionError> {
        Ok(Time::<UTC>::try_from_chrono(dt)?.to::<TT>().reinterpret())
    }

    /// Build a TT [`Time`] in format `F` from a UTC `chrono` timestamp.
    #[track_caller]
    #[inline]
    pub fn from_chrono(dt: DateTime<Utc>) -> Self {
        Self::try_from_chrono(dt).expect("UTC conversion failed; use try_from_chrono")
    }

    /// Like [`Self::try_from_chrono`], but uses an explicit [`TimeContext`].
    #[inline]
    pub fn try_from_chrono_with(
        dt: DateTime<Utc>,
        ctx: &TimeContext,
    ) -> Result<Self, ConversionError> {
        Ok(Time::<UTC>::try_from_chrono_with(dt, ctx)?
            .to::<TT>()
            .reinterpret())
    }

    /// Like [`Self::from_chrono`], but uses an explicit [`TimeContext`].
    #[track_caller]
    #[inline]
    pub fn from_chrono_with(dt: DateTime<Utc>, ctx: &TimeContext) -> Self {
        Self::try_from_chrono_with(dt, ctx)
            .expect("UTC conversion failed; use try_from_chrono_with")
    }

    /// Convert to a UTC `chrono` timestamp (TT → UTC using a default context).
    #[inline]
    pub fn try_to_chrono(self) -> Result<DateTime<Utc>, ConversionError> {
        self.to_j2000s().to::<UTC>().try_to_chrono()
    }

    /// Convert to a UTC `chrono` timestamp (TT → UTC using a default context).
    #[inline]
    pub fn to_chrono(self) -> Option<DateTime<Utc>> {
        self.try_to_chrono().ok()
    }

    /// Like [`Self::try_to_chrono`], but uses an explicit [`TimeContext`].
    #[inline]
    pub fn try_to_chrono_with(self, ctx: &TimeContext) -> Result<DateTime<Utc>, ConversionError> {
        self.to_j2000s().to::<UTC>().try_to_chrono_with(ctx)
    }

    /// Like [`Self::to_chrono`], but uses an explicit [`TimeContext`].
    #[inline]
    pub fn to_chrono_with(self, ctx: &TimeContext) -> Option<DateTime<Utc>> {
        self.try_to_chrono_with(ctx).ok()
    }
}

impl<F: TimeFormat> From<DateTime<Utc>> for Time<TT, F>
where
    F: InfallibleFormatForScale<TT> + InfallibleConversionTarget<TT, Output = Time<TT, F>>,
{
    #[inline]
    fn from(value: DateTime<Utc>) -> Self {
        Self::from_chrono(value)
    }
}
