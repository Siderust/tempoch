// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Built-in external time-format markers.

use crate::foundation::sealed::Sealed;
use crate::qtty::unit;

use super::time_format::TimeFormat;

/// Julian Day (days since noon 1 January 4713 BC on the proleptic Julian
/// calendar). On UTC, JD is the civil UTC coordinate with 86,400 nominal
/// seconds per day and therefore requires fallible, context-aware conversion.
#[derive(Debug, Copy, Clone)]
pub struct JD;
impl Sealed for JD {}
impl TimeFormat for JD {
    type Unit = unit::Day;
    const NAME: &'static str = "JD";
}

/// Modified Julian Day (`JD − 2 400 000.5`). On UTC, MJD is the civil UTC
/// coordinate with 86,400 nominal seconds per day and requires fallible,
/// context-aware conversion. A leap-second instant cannot be encoded.
#[derive(Debug, Copy, Clone)]
pub struct MJD;
impl Sealed for MJD {}
impl TimeFormat for MJD {
    type Unit = unit::Day;
    const NAME: &'static str = "MJD";
}

/// SI seconds since J2000.0 TT (2000-01-01T12:00:00 TT).
#[derive(Debug, Copy, Clone)]
pub struct J2000s;
impl Sealed for J2000s {}
impl TimeFormat for J2000s {
    type Unit = unit::Second;
    const NAME: &'static str = "J2000s";
}

/// POSIX (Unix) seconds since 1970-01-01T00:00:00 UTC.
///
/// Scalar `f64` precision depends on distance from the Unix epoch (about
/// 238 ns per ULP in 2024). Civil history and ambiguity policies match chrono,
/// JD and MJD; leap-second instants cannot be encoded as POSIX seconds.
/// Conversion operates directly on split coordinates without chrono's date
/// range or integer-nanosecond rounding. Split storage still has finite
/// precision and cannot preserve every arbitrarily small `f64` fraction.
#[derive(Debug, Copy, Clone)]
pub struct Unix;
impl Sealed for Unix {}
impl TimeFormat for Unix {
    type Unit = unit::Second;
    const NAME: &'static str = "Unix";
}

/// GPS seconds since 1980-01-06T00:00:00 TAI.
#[derive(Debug, Copy, Clone)]
pub struct GPS;
impl Sealed for GPS {}
impl TimeFormat for GPS {
    type Unit = unit::Second;
    const NAME: &'static str = "GPS";
}
