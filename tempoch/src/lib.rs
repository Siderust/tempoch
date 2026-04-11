// SPDX-License-Identifier: AGPL-3.0-only
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
//! | [`UnixTime`] | Unix / POSIX timestamp |
//! | [`UT`] | Universal Time UT1 (Earth rotation) |
//!
//! # ΔT (Delta T)
//!
//! The difference **ΔT = TT − UT1** is applied automatically by the
//! [`UT`] time scale.  Use `Time::<UT>::new(jd_ut)` for UT-based values.
//! The raw ΔT value (in seconds) is available via [`Time::<UT>::delta_t()`](Time::delta_t).
//!
//! Note: `from_utc()` / `to_utc()` use the compiled UTC/TAI history
//! (`UTC → TAI → TT`) and do **not** go through the ΔT / `UT` scale.
//! Exact UTC conversions are supported from 1961-01-01 onward and preserve
//! positive leap-second labels through chrono's native leap-second encoding.
//! `UnixTime` keeps the usual Unix / POSIX timestamp contract for
//! representable UTC instants; when converted to physical scales it is mapped
//! through `UTC → TAI → TT`, so equal Unix increments are not guaranteed to
//! equal elapsed SI seconds across leap-second insertions.
//!
//! The public `tai_minus_utc()` helper falls back to a fixed 10 s
//! approximation before 1961 for backward compatibility, but exact UTC
//! conversions reject those dates. The compiled modern ΔT series runs through
//! MJD 63871 (`2033-10-01`) and uses a quadratic continuation of the official
//! prediction tail after that point.

pub use tempoch_core::{
    complement_within, intersect_periods, normalize_periods, tai_minus_utc, validate_period_list,
    ConversionError, Interval, InvalidIntervalError, JulianDate, JulianEphemerisDay,
    ModifiedJulianDate, NonFiniteTimeError, Period, PeriodListError, Time, TimeInstant, TimeScale,
    UniversalTime, UnixTime, UtcConversionError, UtcPeriod, DELTA_T_PREDICTION_HORIZON_MJD, GPS,
    JD, JDE, MJD, TAI, TCB, TCG, TDB, TT, UT,
};
