// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Vallés Puig, Ramon

//! Time Module
//!
//! This crate is a façade over `tempoch-core` and re-exports its public API.
//!
//! # Core types
//!
//! - [`Time<S>`] — generic instant parameterised by a [`TimeScale`] marker.
//! - [`TimeScale`] — trait that defines a time scale (epoch offset + conversions).
//! - [`JulianDate`] — type alias for `Time<JD>`.
//! - [`JulianEphemerisDay`] — type alias for `Time<JDE>`.
//! - [`ModifiedJulianDate`] — type alias for `Time<MJD>`.
//! - [`Period<S>`] — a time interval parameterised by a [`TimeScale`] marker.
//! - [`Interval<T>`] — a generic interval over any [`TimeInstant`].
//! - [`TimeInstant`] — trait for points in time usable with [`Interval`].
//!
//! # Time scales
//!
//! The following markers implement [`TimeScale`]:
//!
//! | Marker | Scale |
//! |--------|-------|
//! | [`JD`] | Julian Date |
//! | [`JDE`] | Julian Ephemeris Day |
//! | [`MJD`] | Modified Julian Date |
//! | [`TDB`] | Barycentric Dynamical Time |
//! | [`TT`] | Terrestrial Time |
//! | [`TAI`] | International Atomic Time |
//! | [`TCG`] | Geocentric Coordinate Time |
//! | [`TCB`] | Barycentric Coordinate Time |
//! | [`GPS`] | GPS Time |
//! | [`UnixTime`] | Unix / POSIX time |
//! | [`UT`] | Universal Time (Earth rotation) |
//!
//! # ΔT (Delta T)
//!
//! The difference **ΔT = TT − UT** is applied automatically by the
//! [`UT`] time scale.  Use `Time::<UT>::new(jd_ut)` for UT-based values,
//! or construct any scale via `from_utc()` which routes through `UT` internally.
//! The raw ΔT value (in seconds) is available via [`Time::<UT>::delta_t()`](Time::delta_t).

pub use tempoch_core::{
    complement_within, intersect_periods, tai_minus_utc, Interval, JulianDate, JulianEphemerisDay,
    ModifiedJulianDate, Period, Time, TimeInstant, TimeScale, UniversalTime, UnixTime, UtcPeriod,
    GPS, JD, JDE, MJD, TAI, TCB, TCG, TDB, TT, UT,
};
