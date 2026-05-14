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
use crate::format::{JulianDate, ModifiedJulianDate, UnixTime};
use crate::scale::{TAI, TCB, TCG, TDB, TT, UT1, UTC};
use crate::time::Time;
use qtty::{Day, Second};

/// Identifies a time scale or scalar encoding for dispatch.
///
/// `ScaleKind` is the Rust-native counterpart to C ABI scale identifiers.
/// FFI adapters map their own integer discriminants to `ScaleKind` and then
/// delegate all conversion logic to [`time_tt_from_scalar`] and
/// [`time_tt_to_scalar`] rather than reimplementing the dispatch matrix.
///
/// Variant names follow the `<Format><Scale>` convention: the prefix names
/// the encoding format (e.g. `Jd` = Julian Day, `Mjd` = Modified Julian Day)
/// and the suffix names the time scale (e.g. `Tt`, `Tai`, `Tdb`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleKind {
    /// Julian Day on the TT axis (equivalently: Julian Ephemeris Date). Value in days.
    JdTt,
    /// Modified Julian Day on the TT axis. Value in days.
    MjdTt,
    /// Julian Day on the TDB axis. Value in days.
    JdTdb,
    /// Julian Day on the TAI axis. Value in days.
    JdTai,
    /// Julian Day on the TCG axis. Value in days.
    JdTcg,
    /// Julian Day on the TCB axis. Value in days.
    JdTcb,
    /// Julian Day offset from the GPS epoch, on the TAI axis.
    ///
    /// The unit is **Julian days** (not GPS seconds). A value of `1.0`
    /// represents one Julian day (86 400 s) elapsed since the GPS epoch.
    /// This is distinct from conventional GPS time which is expressed in
    /// integer seconds or (week, seconds-of-week). Divide by 86 400 to
    /// convert from GPS seconds to this encoding.
    JdGps,
    /// Julian Day on the UT1 axis. Value in days.
    JdUt1,
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
        ScaleKind::JdTt => JulianDate::<TT>::try_new(Day::new(value)).map(|e| e.to_time()),
        ScaleKind::MjdTt => ModifiedJulianDate::<TT>::try_new(Day::new(value)).map(|e| e.to_time()),
        ScaleKind::JdTdb => {
            JulianDate::<TDB>::try_new(Day::new(value)).map(|e| e.to_time().to_scale::<TT>())
        }
        ScaleKind::JdTai => {
            JulianDate::<TAI>::try_new(Day::new(value)).map(|e| e.to_time().to_scale::<TT>())
        }
        ScaleKind::JdTcg => {
            JulianDate::<TCG>::try_new(Day::new(value)).map(|e| e.to_time().to_scale::<TT>())
        }
        ScaleKind::JdTcb => {
            JulianDate::<TCB>::try_new(Day::new(value)).map(|e| e.to_time().to_scale::<TT>())
        }
        ScaleKind::JdGps => JulianDate::<TAI>::try_new(GPS_EPOCH_JD_TAI.raw() + Day::new(value))
            .map(|e| e.to_time().to_scale::<TT>()),
        ScaleKind::JdUt1 => JulianDate::<UT1>::try_new(Day::new(value))
            .and_then(|e| e.to_time().to_scale_with::<TT>(ctx)),
        ScaleKind::Unix => UnixTime::try_new(Second::new(value))
            .and_then(|e| e.to_time_with(ctx))
            .map(|t| t.to_scale::<TT>()),
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
    use crate::format::{JD, MJD};
    match kind {
        ScaleKind::JdTt => Ok(tt.to::<JD>().raw() / Day::new(1.0)),
        ScaleKind::MjdTt => Ok(tt.to::<MJD>().raw() / Day::new(1.0)),
        ScaleKind::JdTdb => Ok(tt.to_scale::<TDB>().to::<JD>().raw() / Day::new(1.0)),
        ScaleKind::JdTai => Ok(tt.to_scale::<TAI>().to::<JD>().raw() / Day::new(1.0)),
        ScaleKind::JdTcg => Ok(tt.to_scale::<TCG>().to::<JD>().raw() / Day::new(1.0)),
        ScaleKind::JdTcb => Ok(tt.to_scale::<TCB>().to::<JD>().raw() / Day::new(1.0)),
        ScaleKind::JdGps => {
            Ok((tt.to_scale::<TAI>().to::<JD>().raw() - GPS_EPOCH_JD_TAI.raw()) / Day::new(1.0))
        }
        ScaleKind::JdUt1 => Ok(tt.to_scale_with::<UT1>(ctx)?.to::<JD>().raw() / Day::new(1.0)),
        ScaleKind::Unix => tt
            .to_scale::<UTC>()
            .raw_unix_seconds_with(ctx)
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
