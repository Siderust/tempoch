// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Built-in external time-format markers.

use crate::foundation::sealed::Sealed;
use qtty::unit::{Day as DayUnit, Second as SecondUnit};

use super::time_format::TimeFormat;

/// Julian Day (days since noon 1 January 4713 BC on the proleptic Julian
/// calendar, TT axis by convention).
#[derive(Debug, Copy, Clone)]
pub struct JD;
impl Sealed for JD {}
impl TimeFormat for JD {
    type Unit = DayUnit;
    const NAME: &'static str = "JD";
}

/// Modified Julian Day (`JD − 2 400 000.5`).
#[derive(Debug, Copy, Clone)]
pub struct MJD;
impl Sealed for MJD {}
impl TimeFormat for MJD {
    type Unit = DayUnit;
    const NAME: &'static str = "MJD";
}

/// SI seconds since J2000.0 TT (2000-01-01T12:00:00 TT).
#[derive(Debug, Copy, Clone)]
pub struct J2000s;
impl Sealed for J2000s {}
impl TimeFormat for J2000s {
    type Unit = SecondUnit;
    const NAME: &'static str = "J2000s";
}

/// POSIX (Unix) seconds since 1970-01-01T00:00:00 UTC.
#[derive(Debug, Copy, Clone)]
pub struct Unix;
impl Sealed for Unix {}
impl TimeFormat for Unix {
    type Unit = SecondUnit;
    const NAME: &'static str = "Unix";
}

/// GPS seconds since 1980-01-06T00:00:00 TAI.
#[derive(Debug, Copy, Clone)]
pub struct GPS;
impl Sealed for GPS {}
impl TimeFormat for GPS {
    type Unit = SecondUnit;
    const NAME: &'static str = "GPS";
}
