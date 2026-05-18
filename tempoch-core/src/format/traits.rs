// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Core trait definitions for the format system (`FormatForScale`, etc.).

use crate::earth::context::TimeContext;
use crate::format::TimeFormat;
use crate::foundation::error::ConversionError;
use crate::foundation::sealed::Sealed;
use crate::model::scale::Scale;
use crate::model::time::Time;
use qtty::Quantity;

/// Witness that format `F` can encode and decode instants on scale `S`.
#[allow(private_bounds)]
pub trait FormatForScale<S: Scale>: TimeFormat + Sealed {
    fn try_from_time<Fin: TimeFormat>(
        time: Time<S, Fin>,
        ctx: &TimeContext,
    ) -> Result<Quantity<Self::Unit>, ConversionError>;
    fn try_into_time(
        raw: Quantity<Self::Unit>,
        ctx: &TimeContext,
    ) -> Result<Time<S, Self>, ConversionError>
    where
        Self: Sized;
}

/// Witness that format `F` can encode scale `S` without a [`TimeContext`].
#[allow(private_bounds)]
pub trait InfallibleFormatForScale<S: Scale>: FormatForScale<S> + Sealed {
    fn from_time<Fin: TimeFormat>(time: Time<S, Fin>) -> Quantity<Self::Unit>;
    fn into_time(raw: Quantity<Self::Unit>) -> Time<S, Self>;
}
