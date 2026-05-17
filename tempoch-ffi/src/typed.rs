// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Split-instant and scale/format-aware C ABI.

use crate::context::TempochContext;
use crate::error::TempochStatus;
use crate::time::TempochUtc;
use crate::{catch_panic, QttyQuantity, UnitId};
use qtty::Second;
use tempoch::{
    ConversionError, FormatForScale, GpsTime, J2000Seconds, Time, TimeContext, Unix, UnixTime, JD,
    MJD, TAI, TCB, TCG, TDB, TT, UT1, UTC,
};

/// Scale tags used by the split-instant C ABI.
///
/// cbindgen:prefix-with-name
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempochScaleTag {
    /// Terrestrial Time.
    TT = 0,
    /// International Atomic Time.
    TAI = 1,
    /// Coordinated Universal Time stored on its continuous instant axis.
    UTC = 2,
    /// Universal Time 1.
    UT1 = 3,
    /// Barycentric Dynamical Time.
    TDB = 4,
    /// Geocentric Coordinate Time.
    TCG = 5,
    /// Barycentric Coordinate Time.
    TCB = 6,
}

impl TempochScaleTag {
    /// Attempt to decode a raw ABI discriminant.
    #[inline]
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(Self::TT),
            1 => Some(Self::TAI),
            2 => Some(Self::UTC),
            3 => Some(Self::UT1),
            4 => Some(Self::TDB),
            5 => Some(Self::TCG),
            6 => Some(Self::TCB),
            _ => None,
        }
    }
}

/// Format tags used by the split-instant C ABI.
///
/// cbindgen:prefix-with-name
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempochFormatTag {
    /// Julian Day.
    JD = 0,
    /// Modified Julian Day.
    MJD = 1,
    /// SI seconds since J2000 on the source scale axis.
    J2000Seconds = 2,
    /// POSIX / Unix seconds on the UTC axis.
    Unix = 3,
    /// GPS seconds on the TAI axis.
    GPS = 4,
}

impl TempochFormatTag {
    /// Attempt to decode a raw ABI discriminant.
    #[inline]
    pub fn from_raw(raw: i32) -> Option<Self> {
        match raw {
            0 => Some(Self::JD),
            1 => Some(Self::MJD),
            2 => Some(Self::J2000Seconds),
            3 => Some(Self::Unix),
            4 => Some(Self::GPS),
            _ => None,
        }
    }
}

/// Split J2000-second instant on a scale-specific axis.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TempochTime {
    /// High part of the compensated J2000-second pair.
    pub hi_seconds: f64,
    /// Low residual part of the compensated J2000-second pair.
    pub lo_seconds: f64,
}

impl TempochTime {
    #[inline]
    fn from_time<S: tempoch::Scale>(time: Time<S>) -> Self {
        let (hi, lo) = time.raw_seconds_pair();
        Self {
            hi_seconds: hi.value(),
            lo_seconds: lo.value(),
        }
    }
}

enum ContextBorrow<'a> {
    Borrowed(&'a TimeContext),
    Owned(TimeContext),
}

impl<'a> ContextBorrow<'a> {
    #[inline]
    fn as_ref(&self) -> &TimeContext {
        match self {
            Self::Borrowed(ctx) => ctx,
            Self::Owned(ctx) => ctx,
        }
    }
}

#[inline]
unsafe fn context_or_default<'a>(ptr: *const TempochContext) -> ContextBorrow<'a> {
    if ptr.is_null() {
        ContextBorrow::Owned(TimeContext::new())
    } else {
        ContextBorrow::Borrowed(unsafe { &(*ptr).inner })
    }
}

#[inline]
fn status_from_conversion(err: ConversionError) -> TempochStatus {
    match err {
        ConversionError::Ut1HorizonExceeded => TempochStatus::Ut1HorizonExceeded,
        _ => TempochStatus::ConversionFailed,
    }
}

#[inline]
fn seconds_from_qty(duration: QttyQuantity) -> Result<Second, TempochStatus> {
    duration
        .convert_to(UnitId::Second)
        .map(|q| Second::new(q.value))
        .ok_or(TempochStatus::InvalidDurationUnit)
}

macro_rules! with_scale {
    ($tag:expr, $Scale:ident, $body:block) => {
        match $tag {
            TempochScaleTag::TT => {
                type $Scale = TT;
                $body
            }
            TempochScaleTag::TAI => {
                type $Scale = TAI;
                $body
            }
            TempochScaleTag::UTC => {
                type $Scale = UTC;
                $body
            }
            TempochScaleTag::UT1 => {
                type $Scale = UT1;
                $body
            }
            TempochScaleTag::TDB => {
                type $Scale = TDB;
                $body
            }
            TempochScaleTag::TCG => {
                type $Scale = TCG;
                $body
            }
            TempochScaleTag::TCB => {
                type $Scale = TCB;
                $body
            }
        }
    };
}

#[inline]
fn split_time<S: tempoch::CoordinateScale>(raw: TempochTime) -> Result<Time<S>, ConversionError> {
    Time::<S>::try_from_raw_j2000_seconds_split(
        Second::new(raw.hi_seconds),
        Second::new(raw.lo_seconds),
    )
}

fn decode_unix_to_target(
    raw_seconds: f64,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    if raw_seconds.is_nan() {
        return Err(ConversionError::NonFinite);
    }
    let utc = UnixTime::try_new(Second::new(raw_seconds))?.to_j2000s();
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(utc.to::<TT>())),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(utc.to::<TAI>())),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(utc)),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(utc.to_with::<UT1>(ctx)?)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(utc.to::<TDB>())),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(utc.to::<TCG>())),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(utc.to::<TCB>())),
    }
}

fn decode_gps_to_target(
    raw_seconds: f64,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    if raw_seconds.is_nan() {
        return Err(ConversionError::NonFinite);
    }
    let tai = GpsTime::new(raw_seconds).to_j2000s();
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(tai.to::<TT>())),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(tai)),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(tai.to::<UTC>())),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(tai.to_with::<UT1>(ctx)?)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(tai.to::<TDB>())),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(tai.to::<TCG>())),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(tai.to::<TCB>())),
    }
}

fn encode_unix_from_utc_time(utc: Time<UTC>, ctx: &TimeContext) -> Result<f64, ConversionError> {
    Ok(<Unix as FormatForScale<UTC>>::try_from_time(utc, ctx)?.value())
}

fn encode_gps_from_tai_time(tai: Time<TAI>) -> Result<f64, ConversionError> {
    Ok(tai.to::<tempoch::GPS>().raw().value())
}

fn encode_time_tt(
    raw: TempochTime,
    format: TempochFormatTag,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    let time = split_time::<TT>(raw)?;
    match format {
        TempochFormatTag::JD => Ok(time.to::<JD>().raw().value()),
        TempochFormatTag::MJD => Ok(time.to::<MJD>().raw().value()),
        TempochFormatTag::J2000Seconds => Ok(time.to::<tempoch::J2000s>().raw().value()),
        TempochFormatTag::Unix => encode_unix_from_utc_time(time.to::<UTC>(), ctx),
        TempochFormatTag::GPS => encode_gps_from_tai_time(time.to::<TAI>()),
    }
}

fn encode_time_tai(
    raw: TempochTime,
    format: TempochFormatTag,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    let time = split_time::<TAI>(raw)?;
    match format {
        TempochFormatTag::JD => Ok(time.to::<JD>().raw().value()),
        TempochFormatTag::MJD => Ok(time.to::<MJD>().raw().value()),
        TempochFormatTag::J2000Seconds => Ok(time.to::<tempoch::J2000s>().raw().value()),
        TempochFormatTag::Unix => encode_unix_from_utc_time(time.to::<UTC>(), ctx),
        TempochFormatTag::GPS => encode_gps_from_tai_time(time),
    }
}

fn encode_time_utc(
    raw: TempochTime,
    format: TempochFormatTag,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    let time = split_time::<UTC>(raw)?;
    match format {
        TempochFormatTag::JD => Ok(time.to::<JD>().raw().value()),
        TempochFormatTag::MJD => Ok(time.to::<MJD>().raw().value()),
        TempochFormatTag::J2000Seconds => Ok(time.to::<tempoch::J2000s>().raw().value()),
        TempochFormatTag::Unix => encode_unix_from_utc_time(time, ctx),
        TempochFormatTag::GPS => encode_gps_from_tai_time(time.to::<TAI>()),
    }
}

fn encode_time_ut1(
    raw: TempochTime,
    format: TempochFormatTag,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    let time = split_time::<UT1>(raw)?;
    match format {
        TempochFormatTag::JD => Ok(time.to::<JD>().raw().value()),
        TempochFormatTag::MJD => Ok(time.to::<MJD>().raw().value()),
        TempochFormatTag::J2000Seconds => Ok(time.to::<tempoch::J2000s>().raw().value()),
        TempochFormatTag::Unix => encode_unix_from_utc_time(time.to_with::<UTC>(ctx)?, ctx),
        TempochFormatTag::GPS => encode_gps_from_tai_time(time.to_with::<TAI>(ctx)?),
    }
}

fn encode_time_tdb(
    raw: TempochTime,
    format: TempochFormatTag,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    let time = split_time::<TDB>(raw)?;
    match format {
        TempochFormatTag::JD => Ok(time.to::<JD>().raw().value()),
        TempochFormatTag::MJD => Ok(time.to::<MJD>().raw().value()),
        TempochFormatTag::J2000Seconds => Ok(time.to::<tempoch::J2000s>().raw().value()),
        TempochFormatTag::Unix => encode_unix_from_utc_time(time.to::<UTC>(), ctx),
        TempochFormatTag::GPS => encode_gps_from_tai_time(time.to::<TAI>()),
    }
}

fn encode_time_tcg(
    raw: TempochTime,
    format: TempochFormatTag,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    let time = split_time::<TCG>(raw)?;
    match format {
        TempochFormatTag::JD => Ok(time.to::<JD>().raw().value()),
        TempochFormatTag::MJD => Ok(time.to::<MJD>().raw().value()),
        TempochFormatTag::J2000Seconds => Ok(time.to::<tempoch::J2000s>().raw().value()),
        TempochFormatTag::Unix => encode_unix_from_utc_time(time.to::<UTC>(), ctx),
        TempochFormatTag::GPS => encode_gps_from_tai_time(time.to::<TAI>()),
    }
}

fn encode_time_tcb(
    raw: TempochTime,
    format: TempochFormatTag,
    ctx: &TimeContext,
) -> Result<f64, ConversionError> {
    let time = split_time::<TCB>(raw)?;
    match format {
        TempochFormatTag::JD => Ok(time.to::<JD>().raw().value()),
        TempochFormatTag::MJD => Ok(time.to::<MJD>().raw().value()),
        TempochFormatTag::J2000Seconds => Ok(time.to::<tempoch::J2000s>().raw().value()),
        TempochFormatTag::Unix => encode_unix_from_utc_time(time.to::<UTC>(), ctx),
        TempochFormatTag::GPS => encode_gps_from_tai_time(time.to::<TAI>()),
    }
}

fn scale_convert_tt(
    raw: TempochTime,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    let time = split_time::<TT>(raw)?;
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(time)),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(time.to::<TAI>())),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(time.to::<UTC>())),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(time.to_with::<UT1>(ctx)?)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(time.to::<TDB>())),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(time.to::<TCG>())),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(time.to::<TCB>())),
    }
}

fn scale_convert_tai(
    raw: TempochTime,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    let time = split_time::<TAI>(raw)?;
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(time.to::<TT>())),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(time)),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(time.to::<UTC>())),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(time.to_with::<UT1>(ctx)?)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(time.to::<TDB>())),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(time.to::<TCG>())),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(time.to::<TCB>())),
    }
}

fn scale_convert_utc(
    raw: TempochTime,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    let time = split_time::<UTC>(raw)?;
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(time.to::<TT>())),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(time.to::<TAI>())),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(time)),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(time.to_with::<UT1>(ctx)?)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(time.to::<TDB>())),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(time.to::<TCG>())),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(time.to::<TCB>())),
    }
}

fn scale_convert_ut1(
    raw: TempochTime,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    let time = split_time::<UT1>(raw)?;
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(time.to_with::<TT>(ctx)?)),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(time.to_with::<TAI>(ctx)?)),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(time.to_with::<UTC>(ctx)?)),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(time)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(time.to_with::<TDB>(ctx)?)),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(time.to_with::<TCG>(ctx)?)),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(time.to_with::<TCB>(ctx)?)),
    }
}

fn scale_convert_tdb(
    raw: TempochTime,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    let time = split_time::<TDB>(raw)?;
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(time.to::<TT>())),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(time.to::<TAI>())),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(time.to::<UTC>())),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(time.to_with::<UT1>(ctx)?)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(time)),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(time.to::<TCG>())),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(time.to::<TCB>())),
    }
}

fn scale_convert_tcg(
    raw: TempochTime,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    let time = split_time::<TCG>(raw)?;
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(time.to::<TT>())),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(time.to::<TAI>())),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(time.to::<UTC>())),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(time.to_with::<UT1>(ctx)?)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(time.to::<TDB>())),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(time)),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(time.to::<TCB>())),
    }
}

fn scale_convert_tcb(
    raw: TempochTime,
    target: TempochScaleTag,
    ctx: &TimeContext,
) -> Result<TempochTime, ConversionError> {
    let time = split_time::<TCB>(raw)?;
    match target {
        TempochScaleTag::TT => Ok(TempochTime::from_time(time.to::<TT>())),
        TempochScaleTag::TAI => Ok(TempochTime::from_time(time.to::<TAI>())),
        TempochScaleTag::UTC => Ok(TempochTime::from_time(time.to::<UTC>())),
        TempochScaleTag::UT1 => Ok(TempochTime::from_time(time.to_with::<UT1>(ctx)?)),
        TempochScaleTag::TDB => Ok(TempochTime::from_time(time.to::<TDB>())),
        TempochScaleTag::TCG => Ok(TempochTime::from_time(time.to::<TCG>())),
        TempochScaleTag::TCB => Ok(TempochTime::from_time(time)),
    }
}

/// Validate and normalize a split J2000-second pair.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochTime`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_new(
    hi_seconds: f64,
    lo_seconds: f64,
    out: *mut TempochTime,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        match Time::<TT>::try_from_raw_j2000_seconds_split(
            Second::new(hi_seconds),
            Second::new(lo_seconds),
        ) {
            Ok(time) => {
                unsafe { *out = TempochTime::from_time(time) };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

/// Convert a split instant from one scale to another.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochTime`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_scale_convert(
    value: TempochTime,
    from_scale: i32,
    to_scale: i32,
    context: *const TempochContext,
    out: *mut TempochTime,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let Some(from_scale) = TempochScaleTag::from_raw(from_scale) else {
            return TempochStatus::InvalidScaleId;
        };
        let Some(to_scale) = TempochScaleTag::from_raw(to_scale) else {
            return TempochStatus::InvalidScaleId;
        };
        let ctx = unsafe { context_or_default(context) };
        let converted = match from_scale {
            TempochScaleTag::TT => scale_convert_tt(value, to_scale, ctx.as_ref()),
            TempochScaleTag::TAI => scale_convert_tai(value, to_scale, ctx.as_ref()),
            TempochScaleTag::UTC => scale_convert_utc(value, to_scale, ctx.as_ref()),
            TempochScaleTag::UT1 => scale_convert_ut1(value, to_scale, ctx.as_ref()),
            TempochScaleTag::TDB => scale_convert_tdb(value, to_scale, ctx.as_ref()),
            TempochScaleTag::TCG => scale_convert_tcg(value, to_scale, ctx.as_ref()),
            TempochScaleTag::TCB => scale_convert_tcb(value, to_scale, ctx.as_ref()),
        };
        match converted {
            Ok(time) => {
                unsafe { *out = time };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

/// Encode a split instant in the requested public format.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_to_format(
    value: TempochTime,
    scale: i32,
    format: i32,
    context: *const TempochContext,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let Some(scale) = TempochScaleTag::from_raw(scale) else {
            return TempochStatus::InvalidScaleId;
        };
        let Some(format) = TempochFormatTag::from_raw(format) else {
            return TempochStatus::InvalidFormatId;
        };
        let ctx = unsafe { context_or_default(context) };
        let encoded = match scale {
            TempochScaleTag::TT => encode_time_tt(value, format, ctx.as_ref()),
            TempochScaleTag::TAI => encode_time_tai(value, format, ctx.as_ref()),
            TempochScaleTag::UTC => encode_time_utc(value, format, ctx.as_ref()),
            TempochScaleTag::UT1 => encode_time_ut1(value, format, ctx.as_ref()),
            TempochScaleTag::TDB => encode_time_tdb(value, format, ctx.as_ref()),
            TempochScaleTag::TCG => encode_time_tcg(value, format, ctx.as_ref()),
            TempochScaleTag::TCB => encode_time_tcb(value, format, ctx.as_ref()),
        };
        match encoded {
            Ok(raw) => {
                unsafe { *out = raw };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

/// Decode a split instant from the requested public format.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochTime`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_from_format(
    raw: f64,
    scale: i32,
    format: i32,
    context: *const TempochContext,
    out: *mut TempochTime,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let Some(scale) = TempochScaleTag::from_raw(scale) else {
            return TempochStatus::InvalidScaleId;
        };
        let Some(format) = TempochFormatTag::from_raw(format) else {
            return TempochStatus::InvalidFormatId;
        };
        let ctx = unsafe { context_or_default(context) };
        let decoded = match format {
            TempochFormatTag::JD => with_scale!(scale, Scale, {
                if raw.is_nan() {
                    Err(ConversionError::NonFinite)
                } else {
                    Ok(TempochTime::from_time(
                        tempoch::JulianDate::<Scale>::new(raw).to_j2000s(),
                    ))
                }
            }),
            TempochFormatTag::MJD => with_scale!(scale, Scale, {
                if raw.is_nan() {
                    Err(ConversionError::NonFinite)
                } else {
                    Ok(TempochTime::from_time(
                        tempoch::ModifiedJulianDate::<Scale>::new(raw).to_j2000s(),
                    ))
                }
            }),
            TempochFormatTag::J2000Seconds => with_scale!(scale, Scale, {
                if raw.is_nan() {
                    Err(ConversionError::NonFinite)
                } else {
                    Ok(TempochTime::from_time(
                        J2000Seconds::<Scale>::new(raw).to_j2000s(),
                    ))
                }
            }),
            TempochFormatTag::Unix => decode_unix_to_target(raw, scale, ctx.as_ref()),
            TempochFormatTag::GPS => decode_gps_to_target(raw, scale, ctx.as_ref()),
        };
        match decoded {
            Ok(time) => {
                unsafe { *out = time };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

/// Build a UTC-axis split instant from a civil calendar label.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochTime`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_from_civil(
    civil: TempochUtc,
    context: *const TempochContext,
    out: *mut TempochTime,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let Some(dt) = civil.into_chrono() else {
            return TempochStatus::ConversionFailed;
        };
        let ctx = unsafe { context_or_default(context) };
        match Time::<UTC>::try_from_chrono_with(dt, ctx.as_ref()) {
            Ok(time) => {
                unsafe { *out = TempochTime::from_time(time) };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

/// Convert a split instant on the UTC axis to a civil calendar label.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochUtc`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_to_civil(
    value: TempochTime,
    context: *const TempochContext,
    out: *mut TempochUtc,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let ctx = unsafe { context_or_default(context) };
        match split_time::<UTC>(value).and_then(|time| time.try_to_chrono_with(ctx.as_ref())) {
            Ok(dt) => {
                unsafe { *out = TempochUtc::from_chrono(&dt) };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

/// Shift a split instant by a duration convertible to seconds.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochTime`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_add_seconds(
    value: TempochTime,
    duration: QttyQuantity,
    out: *mut TempochTime,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let seconds = match seconds_from_qty(duration) {
            Ok(v) => v,
            Err(status) => return status,
        };
        match Time::<TT>::try_from_raw_j2000_seconds_split(
            Second::new(value.hi_seconds),
            Second::new(value.lo_seconds),
        ) {
            Ok(time) => {
                unsafe { *out = TempochTime::from_time(time + seconds) };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

/// Return `lhs - rhs` in SI seconds.
///
/// # Safety
/// `out` must be a valid, writable pointer to `double`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_difference_seconds(
    lhs: TempochTime,
    rhs: TempochTime,
    out: *mut f64,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let lhs_time = match Time::<TT>::try_from_raw_j2000_seconds_split(
            Second::new(lhs.hi_seconds),
            Second::new(lhs.lo_seconds),
        ) {
            Ok(v) => v,
            Err(err) => return status_from_conversion(err),
        };
        let rhs_time = match Time::<TT>::try_from_raw_j2000_seconds_split(
            Second::new(rhs.hi_seconds),
            Second::new(rhs.lo_seconds),
        ) {
            Ok(v) => v,
            Err(err) => return status_from_conversion(err),
        };
        unsafe { *out = (lhs_time - rhs_time).value() };
        TempochStatus::Ok
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    fn utc_j2000() -> TempochUtc {
        TempochUtc {
            year: 2000,
            month: 1,
            day: 1,
            hour: 12,
            minute: 0,
            second: 0,
            nanosecond: 0,
        }
    }

    #[test]
    fn roundtrip_tt_jd_through_split_time() {
        let mut value = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_from_format(2_451_545.0, 0, 0, ptr::null(), &mut value) },
            TempochStatus::Ok
        );

        let mut jd = 0.0;
        assert_eq!(
            unsafe { tempoch_time_to_format(value, 0, 0, ptr::null(), &mut jd) },
            TempochStatus::Ok
        );
        assert!((jd - 2_451_545.0).abs() < 1e-12);
    }

    #[test]
    fn tag_decoders_reject_invalid_ids() {
        assert_eq!(TempochScaleTag::from_raw(0), Some(TempochScaleTag::TT));
        assert_eq!(TempochScaleTag::from_raw(6), Some(TempochScaleTag::TCB));
        assert_eq!(TempochScaleTag::from_raw(-1), None);
        assert_eq!(TempochFormatTag::from_raw(0), Some(TempochFormatTag::JD));
        assert_eq!(TempochFormatTag::from_raw(4), Some(TempochFormatTag::GPS));
        assert_eq!(TempochFormatTag::from_raw(99), None);
    }

    #[test]
    fn unix_decode_to_ut1_requires_context_but_defaults() {
        let mut value = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_from_format(946_727_935.816, 3, 3, ptr::null(), &mut value) },
            TempochStatus::Ok
        );
    }

    #[test]
    fn typed_time_validation_paths() {
        let mut value = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_new(0.0, 0.0, &mut value) },
            TempochStatus::Ok
        );

        let mut untouched = TempochTime {
            hi_seconds: 123.0,
            lo_seconds: 456.0,
        };
        assert_eq!(
            unsafe { tempoch_time_new(f64::NAN, 0.0, &mut untouched) },
            TempochStatus::ConversionFailed
        );
        assert_eq!(untouched.hi_seconds, 123.0);
        assert_eq!(untouched.lo_seconds, 456.0);

        let mut added = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe {
                tempoch_time_add_seconds(
                    TempochTime {
                        hi_seconds: 0.0,
                        lo_seconds: 0.0,
                    },
                    QttyQuantity::new(1.0, UnitId::Day),
                    &mut added,
                )
            },
            TempochStatus::Ok
        );
        assert!((added.hi_seconds - 86_400.0).abs() < 1e-12);

        let mut diff = 0.0;
        assert_eq!(
            unsafe {
                tempoch_time_difference_seconds(
                    TempochTime {
                        hi_seconds: 10.0,
                        lo_seconds: 0.0,
                    },
                    TempochTime {
                        hi_seconds: 7.5,
                        lo_seconds: 0.0,
                    },
                    &mut diff,
                )
            },
            TempochStatus::Ok
        );
        assert!((diff - 2.5).abs() < 1e-12);

        assert_eq!(
            unsafe {
                tempoch_time_add_seconds(
                    TempochTime {
                        hi_seconds: 0.0,
                        lo_seconds: 0.0,
                    },
                    QttyQuantity::new(1.0, UnitId::Meter),
                    &mut added,
                )
            },
            TempochStatus::InvalidDurationUnit
        );

        assert_eq!(
            unsafe { tempoch_time_new(0.0, 0.0, ptr::null_mut(),) },
            TempochStatus::NullPointer
        );
    }

    #[test]
    fn typed_format_and_scale_roundtrips_cover_unix_gps_and_ut1() {
        let mut unix_time = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_from_format(0.0, 2, 3, ptr::null(), &mut unix_time) },
            TempochStatus::Ok
        );
        let mut unix_raw = 0.0;
        assert_eq!(
            unsafe { tempoch_time_to_format(unix_time, 2, 3, ptr::null(), &mut unix_raw) },
            TempochStatus::Ok
        );
        assert!(unix_raw.abs() < 1e-5);

        let mut gps_time = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_from_format(0.0, 0, 4, ptr::null(), &mut gps_time) },
            TempochStatus::Ok
        );
        let mut gps_raw = 0.0;
        assert_eq!(
            unsafe { tempoch_time_to_format(gps_time, 0, 4, ptr::null(), &mut gps_raw) },
            TempochStatus::Ok
        );
        assert!(gps_raw.abs() < 1e-5);

        let mut ut1_time = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_from_format(2_451_545.0, 3, 0, ptr::null(), &mut ut1_time) },
            TempochStatus::Ok
        );
        let mut jd = 0.0;
        assert_eq!(
            unsafe { tempoch_time_to_format(ut1_time, 3, 0, ptr::null(), &mut jd) },
            TempochStatus::Ok
        );
        assert!((jd - 2_451_545.0).abs() < 1e-12);

        let mut converted = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_scale_convert(ut1_time, 3, 0, ptr::null(), &mut converted) },
            TempochStatus::Ok
        );

        let mut future_utc = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_from_format(2_465_000.0, 2, 0, ptr::null(), &mut future_utc) },
            TempochStatus::Ok
        );
        let mut future_ut1 = TempochTime {
            hi_seconds: -1.0,
            lo_seconds: -1.0,
        };
        assert_eq!(
            unsafe { tempoch_time_scale_convert(future_utc, 2, 3, ptr::null(), &mut future_ut1) },
            TempochStatus::Ut1HorizonExceeded
        );
    }

    #[test]
    fn typed_civil_roundtrip_and_invalid_inputs() {
        let mut value = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe { tempoch_time_from_civil(utc_j2000(), ptr::null(), &mut value) },
            TempochStatus::Ok
        );

        let mut civil = TempochUtc {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            nanosecond: 0,
        };
        assert_eq!(
            unsafe { tempoch_time_to_civil(value, ptr::null(), &mut civil) },
            TempochStatus::Ok
        );
        assert_eq!(civil.year, 2000);
        assert_eq!(civil.month, 1);
        assert_eq!(civil.day, 1);

        let mut invalid = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        assert_eq!(
            unsafe {
                tempoch_time_from_civil(
                    TempochUtc {
                        year: 2000,
                        month: 13,
                        day: 1,
                        hour: 0,
                        minute: 0,
                        second: 0,
                        nanosecond: 0,
                    },
                    ptr::null(),
                    &mut invalid,
                )
            },
            TempochStatus::ConversionFailed
        );

        assert_eq!(
            unsafe { tempoch_time_to_format(value, 0, 99, ptr::null(), &mut 0.0) },
            TempochStatus::InvalidFormatId
        );
        assert_eq!(
            unsafe { tempoch_time_from_format(0.0, 99, 0, ptr::null(), &mut invalid) },
            TempochStatus::InvalidScaleId
        );
        assert_eq!(
            unsafe { tempoch_time_from_format(0.0, 0, 99, ptr::null(), &mut invalid) },
            TempochStatus::InvalidFormatId
        );
        assert_eq!(
            unsafe { tempoch_time_scale_convert(value, 99, 0, ptr::null(), &mut invalid) },
            TempochStatus::InvalidScaleId
        );
    }
}
