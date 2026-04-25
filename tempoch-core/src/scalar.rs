// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scalar-value adapter for time scale dispatch.
//!
//! This module is the single authoritative conversion matrix between `f64`
//! scalar values in various scales and [`Time<TT>`]. FFI and other
//! scalar-oriented layers should delegate here instead of reimplementing the
//! dispatch logic themselves.
//!
//! # Usage
//!
//! An FFI crate maps its own integer discriminants to [`ScaleKind`] and then
//! calls [`time_tt_from_scalar`] / [`time_tt_to_scalar`] for all roundtrips
//! through the canonical TT axis. Arithmetic helpers
//! ([`scalar_difference_in_days`], [`scalar_add_days`]) handle the
//! seconds-vs-days distinction for the [`ScaleKind::Unix`] encoding.

use crate::constats::GPS_EPOCH_JD_TAI;
use crate::context::TimeContext;
use crate::error::ConversionError;
use crate::scale::{TAI, TCB, TCG, TDB, TT, UT1, UTC};
use crate::time::Time;
use qtty::{Day, Second};

/// Identifies a time scale or scalar encoding for dispatch.
///
/// `ScaleKind` is the Rust-native counterpart to C ABI scale identifiers.
/// FFI adapters map their own integer discriminants to `ScaleKind` and then
/// delegate all conversion logic to [`time_tt_from_scalar`] and
/// [`time_tt_to_scalar`] rather than reimplementing the dispatch matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleKind {
    /// Julian Day (TT) — equivalently Julian Ephemeris Date (JDE). Value in days.
    JdTt,
    /// Modified Julian Day on the TT axis. Value in days.
    MjdTt,
    /// Barycentric Dynamical Time, Julian days on the TDB axis.
    Tdb,
    /// International Atomic Time, Julian days on the TAI axis.
    Tai,
    /// Geocentric Coordinate Time, Julian days on the TCG axis.
    Tcg,
    /// Barycentric Coordinate Time, Julian days on the TCB axis.
    Tcb,
    /// GPS days since [`GPS_EPOCH_JD_TAI`] on the TAI axis.
    ///
    /// The unit is **Julian days** (not GPS seconds). A value of `1.0`
    /// represents one Julian day (86 400 s) elapsed since the GPS epoch.
    /// This is distinct from conventional GPS time which is expressed in
    /// integer seconds or (week, seconds-of-week). Divide by 86 400 to
    /// convert from GPS seconds to this representation.
    GpsDays,
    /// Universal Time UT1, Julian days on the UT1 axis.
    Ut1,
    /// Unix / POSIX time in seconds since 1970-01-01T00:00:00 UTC.
    Unix,
}

/// Convert a scalar in the given scale to [`Time<TT>`].
///
/// This is the single authoritative entry point for scalar → `Time<TT>`.
/// For context-free scales the `ctx` argument is unused; for
/// [`ScaleKind::Ut1`] `ctx` supplies the ΔT table used by the UT1→TT
/// conversion.
#[inline]
pub fn time_tt_from_scalar(
    value: f64,
    kind: ScaleKind,
    ctx: &TimeContext,
) -> Result<Time<TT>, ConversionError> {
    match kind {
        ScaleKind::JdTt => Time::<TT>::from_julian_days(Day::new(value)),
        ScaleKind::MjdTt => Time::<TT>::from_modified_julian_days(Day::new(value)),
        ScaleKind::Tdb => {
            Time::<TDB>::from_julian_days(Day::new(value)).map(|t| t.to_scale::<TT>())
        }
        ScaleKind::Tai => {
            Time::<TAI>::from_julian_days(Day::new(value)).map(|t| t.to_scale::<TT>())
        }
        ScaleKind::Tcg => {
            Time::<TCG>::from_julian_days(Day::new(value)).map(|t| t.to_scale::<TT>())
        }
        ScaleKind::Tcb => {
            Time::<TCB>::from_julian_days(Day::new(value)).map(|t| t.to_scale::<TT>())
        }
        ScaleKind::GpsDays => Time::<TAI>::from_julian_days(GPS_EPOCH_JD_TAI + Day::new(value))
            .map(|t| t.to_scale::<TT>()),
        ScaleKind::Ut1 => {
            Time::<UT1>::from_julian_days(Day::new(value)).and_then(|t| t.to_scale_with::<TT>(ctx))
        }
        ScaleKind::Unix => {
            Time::<UTC>::from_unix_seconds_with(Second::new(value), ctx).map(|t| t.to_scale::<TT>())
        }
    }
}

/// Convert a [`Time<TT>`] value to a scalar in the given scale.
///
/// This is the single authoritative entry point for `Time<TT>` → scalar.
/// For context-free scales the `ctx` argument is unused; for
/// [`ScaleKind::Ut1`] and [`ScaleKind::Unix`] `ctx` supplies the ΔT /
/// UTC-TAI table.
#[inline]
pub fn time_tt_to_scalar(
    tt: Time<TT>,
    kind: ScaleKind,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    match kind {
        ScaleKind::JdTt => Ok(tt.julian_days() / Day::new(1.0)),
        ScaleKind::MjdTt => Ok(tt.modified_julian_days() / Day::new(1.0)),
        ScaleKind::Tdb => Ok(tt.to_scale::<TDB>().julian_days() / Day::new(1.0)),
        ScaleKind::Tai => Ok(tt.to_scale::<TAI>().julian_days() / Day::new(1.0)),
        ScaleKind::Tcg => Ok(tt.to_scale::<TCG>().julian_days() / Day::new(1.0)),
        ScaleKind::Tcb => Ok(tt.to_scale::<TCB>().julian_days() / Day::new(1.0)),
        ScaleKind::GpsDays => {
            Ok((tt.to_scale::<TAI>().julian_days() - GPS_EPOCH_JD_TAI) / Day::new(1.0))
        }
        ScaleKind::Ut1 => Ok(tt.to_scale_with::<UT1>(ctx)?.julian_days() / Day::new(1.0)),
        ScaleKind::Unix => tt
            .to_scale::<UTC>()
            .unix_seconds_with(ctx)
            .map(|s| s / Second::new(1.0)),
    }
}

/// Compute the difference between two scalar values in the same scale, in days.
///
/// For [`ScaleKind::Unix`] (seconds), the raw difference is converted to days.
/// For all other scales the raw difference already represents days.
#[inline]
pub fn scalar_difference_in_days(lhs: f64, rhs: f64, kind: ScaleKind) -> f64 {
    match kind {
        ScaleKind::Unix => Second::new(lhs - rhs).to::<qtty::unit::Day>() / Day::new(1.0),
        _ => lhs - rhs,
    }
}

/// Add a day-valued duration to a scalar in the given scale.
///
/// For [`ScaleKind::Unix`] (seconds) the duration is converted to seconds
/// before adding. For all other scales the duration is added directly as days.
#[inline]
pub fn scalar_add_days(value: f64, days: Day, kind: ScaleKind) -> f64 {
    match kind {
        ScaleKind::Unix => value + days.to::<qtty::unit::Second>() / Second::new(1.0),
        _ => value + days / Day::new(1.0),
    }
}
