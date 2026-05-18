// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! `FormatForScale` / `InfallibleFormatForScale` for built-in format markers.

use super::markers::{J2000s, Unix, GPS, JD, MJD};
use super::traits::{FormatForScale, InfallibleFormatForScale};
use crate::earth::context::TimeContext;
use crate::encoding::{day_to_j2000_seconds, j2000_seconds_to_day};
use crate::format::TimeFormat;
use crate::foundation::error::ConversionError;
use crate::model::scale::{CoordinateScale, TAI, UTC};
use crate::model::time::Time;
use qtty::{Day, Second};

impl<S: CoordinateScale> FormatForScale<S> for J2000s {
    #[inline]
    fn try_from_time<Fin: TimeFormat>(
        time: Time<S, Fin>,
        _ctx: &TimeContext,
    ) -> Result<Second, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<S>>::from_time(time))
    }

    #[inline]
    fn try_into_time(raw: Second, _ctx: &TimeContext) -> Result<Time<S, Self>, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<S>>::into_time(raw))
    }
}

impl<S: CoordinateScale> InfallibleFormatForScale<S> for J2000s {
    #[inline]
    fn from_time<Fin: TimeFormat>(time: Time<S, Fin>) -> Second {
        time.to_j2000s().raw_j2000_seconds()
    }

    #[inline]
    fn into_time(raw: Second) -> Time<S, Self> {
        Time::<S, J2000s>::from_raw_j2000_seconds(raw).expect("finite J2000 seconds must decode")
    }
}

impl<S: CoordinateScale> FormatForScale<S> for JD {
    #[inline]
    fn try_from_time<Fin: TimeFormat>(
        time: Time<S, Fin>,
        _ctx: &TimeContext,
    ) -> Result<Day, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<S>>::from_time(time))
    }

    #[inline]
    fn try_into_time(raw: Day, _ctx: &TimeContext) -> Result<Time<S, Self>, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<S>>::into_time(raw))
    }
}

impl<S: CoordinateScale> InfallibleFormatForScale<S> for JD {
    #[inline]
    fn from_time<Fin: TimeFormat>(time: Time<S, Fin>) -> Day {
        j2000_seconds_to_day::<JD>(time.to_j2000s().raw_j2000_seconds())
    }

    #[inline]
    fn into_time(raw: Day) -> Time<S, Self> {
        Time::<S, J2000s>::from_raw_j2000_seconds(day_to_j2000_seconds::<JD>(raw))
            .expect("finite Julian date must decode")
            .reinterpret()
    }
}

impl<S: CoordinateScale> FormatForScale<S> for MJD {
    #[inline]
    fn try_from_time<Fin: TimeFormat>(
        time: Time<S, Fin>,
        _ctx: &TimeContext,
    ) -> Result<Day, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<S>>::from_time(time))
    }

    #[inline]
    fn try_into_time(raw: Day, _ctx: &TimeContext) -> Result<Time<S, Self>, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<S>>::into_time(raw))
    }
}

impl<S: CoordinateScale> InfallibleFormatForScale<S> for MJD {
    #[inline]
    fn from_time<Fin: TimeFormat>(time: Time<S, Fin>) -> Day {
        j2000_seconds_to_day::<MJD>(time.to_j2000s().raw_j2000_seconds())
    }

    #[inline]
    fn into_time(raw: Day) -> Time<S, Self> {
        Time::<S, J2000s>::from_raw_j2000_seconds(day_to_j2000_seconds::<MJD>(raw))
            .expect("finite Modified Julian date must decode")
            .reinterpret()
    }
}

impl FormatForScale<UTC> for Unix {
    #[inline]
    fn try_from_time<Fin: TimeFormat>(
        time: Time<UTC, Fin>,
        ctx: &TimeContext,
    ) -> Result<Second, ConversionError> {
        time.to_j2000s().raw_unix_seconds_with(ctx)
    }

    #[inline]
    fn try_into_time(raw: Second, ctx: &TimeContext) -> Result<Time<UTC, Self>, ConversionError> {
        Time::<UTC, J2000s>::from_raw_unix_seconds_with(raw, ctx).map(|t| t.reinterpret())
    }
}

impl FormatForScale<TAI> for GPS {
    #[inline]
    fn try_from_time<Fin: TimeFormat>(
        time: Time<TAI, Fin>,
        _ctx: &TimeContext,
    ) -> Result<Second, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<TAI>>::from_time(time))
    }

    #[inline]
    fn try_into_time(raw: Second, _ctx: &TimeContext) -> Result<Time<TAI, Self>, ConversionError> {
        Ok(<Self as InfallibleFormatForScale<TAI>>::into_time(raw))
    }
}

impl InfallibleFormatForScale<TAI> for GPS {
    #[inline]
    fn from_time<Fin: TimeFormat>(time: Time<TAI, Fin>) -> Second {
        time.to_j2000s().raw_gps_seconds()
    }

    #[inline]
    fn into_time(raw: Second) -> Time<TAI, Self> {
        Time::<TAI, J2000s>::from_raw_gps_seconds(raw)
            .expect("finite GPS seconds must decode")
            .reinterpret()
    }
}
