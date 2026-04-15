// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion matrix (RFC 0001 §7, §11).
//!
//! Three disjoint witness traits enumerate which axis pairs are valid:
//!
//! * [`InfallibleConvertible`] — exact/affine + context-free closed-form
//!   (always succeeds).
//! * [`FallibleConvertible`]    — depends on compiled UTC–TAI history; can
//!   fail with a `ConversionError`.
//! * [`ContextConvertible`]     — requires a `TimeContext` (UT1 routes).
//!
//! Each witness carries the conversion itself as a method, so `Time<A, R>`
//! never has to pattern-match on axis at runtime — the compiler inlines the
//! concrete path.

// Witness trait methods are `#[doc(hidden)]` — their `Storage<_>` parameters
// leak through the trait API by construction but are not part of the public
// surface.
#![allow(private_interfaces)]

use super::axis::{Axis, TAI, TCB, TCG, TDB, TT, UT1, UTC};
use super::constats::{
    IAU_TIME_EPOCH_T0_JD, L_B, L_G, TDB0, TT_MINUS_TAI,
};
use super::context::TimeContext;
use super::delta_t::{delta_t_seconds, DELTA_T_PREDICTION_HORIZON_MJD};
use super::encoding::{
    j2000_seconds_to_jd, jd_to_j2000_seconds, jd_to_julian_centuries, jd_to_mjd,
    mjd_to_j2000_seconds,
};
use super::error::ConversionError;
use super::sealed::Sealed;
use super::storage::Storage;
use crate::generated::time_data::{UtcTaiSegment, UTC_TAI_SEGMENTS};
use crate::generated::{PRE_1961_TAI_MINUS_UTC_APPROX, UTC_TAI_HISTORY_START_MJD};
use qtty::time::{Days, Seconds};
use qtty::unit::Day;

/// Unix epoch (1970-01-01T00:00:00 UTC) expressed as seconds since J2000 TT
/// on the TAI axis. Relies on the compiled UTC-TAI history.
#[allow(dead_code)]
#[inline]
pub(crate) fn unix_epoch_tai_secs() -> Seconds {
    // TAI-UTC at MJD 40587 per the compiled history.
    let ls = try_tai_minus_utc_mjd(crate::constats::UNIX_EPOCH_MJD)
        .unwrap_or(PRE_1961_TAI_MINUS_UTC_APPROX);
    mjd_to_j2000_seconds(crate::constats::UNIX_EPOCH_MJD) + ls + TT_MINUS_TAI
}

// ── UTC-TAI history lookup ───────────────────────────────────────────────

#[inline]
pub(crate) fn utc_offset_seconds_in_segment(mjd_utc: Days, segment: UtcTaiSegment) -> Seconds {
    segment.offset_at(mjd_utc)
}

#[inline]
pub(crate) fn utc_mjd_to_tt_mjd_in_segment(mjd_utc: Days, segment: UtcTaiSegment) -> Days {
    mjd_utc + (utc_offset_seconds_in_segment(mjd_utc, segment) + TT_MINUS_TAI).to::<Day>()
}

#[inline]
pub(crate) fn tt_mjd_to_utc_mjd_in_segment(mjd_tt: Days, segment: UtcTaiSegment) -> Days {
    let scale = Days::new(1.0) + Seconds::new(segment.slope_seconds_per_day).to::<Day>();
    let ref_days = segment.reference_mjd_days() / Days::new(1.0);
    let offset_days = (Seconds::new(segment.base_seconds)
        - Seconds::new(segment.slope_seconds_per_day) * ref_days
        + TT_MINUS_TAI)
        .to::<Day>();
    Days::new((mjd_tt - offset_days) / scale)
}

/// Binary search: TAI − UTC at a UTC-axis MJD. Returns `None` pre-1961.
#[inline]
pub(crate) fn try_tai_minus_utc_mjd(mjd_utc: Days) -> Option<Seconds> {
    if mjd_utc < UTC_TAI_HISTORY_START_MJD {
        return None;
    }
    let mut lo = 0usize;
    let mut hi = UTC_TAI_SEGMENTS.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if UTC_TAI_SEGMENTS[mid].start_mjd_days() <= mjd_utc {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let segment = UTC_TAI_SEGMENTS[lo - 1];
    Some(utc_offset_seconds_in_segment(mjd_utc, segment))
}

// ── TDB ↔ TT: Fairhead–Bretagnon 4-term ──────────────────────────────────

#[inline]
fn tdb_minus_tt_seconds(jd_tt: Days) -> Seconds {
    let t = jd_to_julian_centuries(jd_tt);
    let m_e = (357.5291092_f64 + 35_999.0502909 * t).to_radians();
    let m_j = (246.4512_f64 + 3_035.2335 * t).to_radians();
    let d = (297.8502042_f64 + 445_267.1115168 * t).to_radians();
    let om = (125.0445550_f64 - 1_934.1362091 * t).to_radians();
    Seconds::new(
        0.001_657 * m_e.sin()
            + 0.000_022 * (d - m_e).sin()
            + 0.000_014 * (2.0 * d).sin()
            + 0.000_005 * m_j.sin()
            + 0.000_005 * om.sin(),
    )
}

// ── Public witness traits ────────────────────────────────────────────────

/// Witness that converting from `Self` to `A2` is an infallible closed-form
/// operation.
pub trait InfallibleConvertible<A2: Axis>: Axis + Sealed {
    #[doc(hidden)]
    fn convert(src: Storage<Self>) -> Storage<A2>;
}

/// Witness that converting from `Self` to `A2` uses the compiled UTC–TAI
/// history and may fail (e.g. pre-1961).
pub trait FallibleConvertible<A2: Axis>: Axis + Sealed {
    #[doc(hidden)]
    fn try_convert(src: Storage<Self>) -> Result<Storage<A2>, ConversionError>;
}

/// Witness that converting from `Self` to `A2` requires an explicit
/// `TimeContext`.
pub trait ContextConvertible<A2: Axis>: Axis + Sealed {
    #[doc(hidden)]
    fn convert_with(src: Storage<Self>, ctx: &TimeContext) -> Result<Storage<A2>, ConversionError>;
}

// ── Identity ─────────────────────────────────────────────────────────────

macro_rules! identity_infallible {
    ($($axis:ty),+ $(,)?) => {
        $(
            impl InfallibleConvertible<$axis> for $axis {
                #[inline]
                fn convert(src: Storage<$axis>) -> Storage<$axis> { src }
            }
        )+
    };
}
identity_infallible!(TAI, TT, TDB, TCG, TCB, UTC, UT1);

// ── Continuous pairs: TAI ↔ TT ↔ TDB ↔ TCG ↔ TCB ─────────────────────────

impl InfallibleConvertible<TT> for TAI {
    #[inline]
    fn convert(src: Storage<TAI>) -> Storage<TT> {
        Storage::new_unchecked(src.seconds + TT_MINUS_TAI, false)
    }
}
impl InfallibleConvertible<TAI> for TT {
    #[inline]
    fn convert(src: Storage<TT>) -> Storage<TAI> {
        Storage::new_unchecked(src.seconds - TT_MINUS_TAI, false)
    }
}

impl InfallibleConvertible<TDB> for TT {
    #[inline]
    fn convert(src: Storage<TT>) -> Storage<TDB> {
        let jd_tt: Days = j2000_seconds_to_jd(src.seconds);
        let delta: Seconds = tdb_minus_tt_seconds(jd_tt);
        Storage::new_unchecked(src.seconds + delta, false)
    }
}
impl InfallibleConvertible<TT> for TDB {
    #[inline]
    fn convert(src: Storage<TDB>) -> Storage<TT> {
        let mut jd_tt: Days = j2000_seconds_to_jd(src.seconds);
        for _ in 0..2 {
            jd_tt = j2000_seconds_to_jd(src.seconds - tdb_minus_tt_seconds(jd_tt));
        }
        let delta: Seconds = tdb_minus_tt_seconds(jd_tt);
        Storage::new_unchecked(src.seconds - delta, false)
    }
}

impl InfallibleConvertible<TCG> for TT {
    #[inline]
    fn convert(src: Storage<TT>) -> Storage<TCG> {
        let t0_secs_tt = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let tcg_secs = src.seconds + L_G * (src.seconds - t0_secs_tt) / (1.0 - L_G);
        Storage::new_unchecked(tcg_secs, false)
    }
}
impl InfallibleConvertible<TT> for TCG {
    #[inline]
    fn convert(src: Storage<TCG>) -> Storage<TT> {
        let t0_secs_tt = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let tt_secs = src.seconds - L_G * (src.seconds - t0_secs_tt);
        Storage::new_unchecked(tt_secs, false)
    }
}

impl InfallibleConvertible<TCB> for TDB {
    #[inline]
    fn convert(src: Storage<TDB>) -> Storage<TCB> {
        let t0_secs_tt = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = src.seconds - t0_secs_tt - TDB0;
        let tcb_secs = t0_secs_tt + delta / (1.0 - L_B);
        Storage::new_unchecked(tcb_secs, false)
    }
}
impl InfallibleConvertible<TDB> for TCB {
    #[inline]
    fn convert(src: Storage<TCB>) -> Storage<TDB> {
        let t0_secs_tt = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = src.seconds - t0_secs_tt;
        let tdb_secs = t0_secs_tt + (1.0 - L_B) * delta + TDB0;
        Storage::new_unchecked(tdb_secs, false)
    }
}

// Transitive pairs through TT.
macro_rules! through_tt {
    ($from:ty, $to:ty) => {
        impl InfallibleConvertible<$to> for $from {
            #[inline]
            fn convert(src: Storage<$from>) -> Storage<$to> {
                let tt = <$from as InfallibleConvertible<TT>>::convert(src);
                <TT as InfallibleConvertible<$to>>::convert(tt)
            }
        }
    };
}
through_tt!(TAI, TDB);
through_tt!(TDB, TAI);
through_tt!(TAI, TCG);
through_tt!(TCG, TAI);
through_tt!(TAI, TCB);
through_tt!(TCB, TAI);
through_tt!(TDB, TCG);
through_tt!(TCG, TDB);
through_tt!(TCG, TCB);
through_tt!(TCB, TCG);

impl InfallibleConvertible<TCB> for TT {
    #[inline]
    fn convert(src: Storage<TT>) -> Storage<TCB> {
        let tdb = <TT as InfallibleConvertible<TDB>>::convert(src);
        <TDB as InfallibleConvertible<TCB>>::convert(tdb)
    }
}
impl InfallibleConvertible<TT> for TCB {
    #[inline]
    fn convert(src: Storage<TCB>) -> Storage<TT> {
        let tdb = <TCB as InfallibleConvertible<TDB>>::convert(src);
        <TDB as InfallibleConvertible<TT>>::convert(tdb)
    }
}

// ── UTC ↔ TAI (and transitive UTC ↔ TT/TDB/TCG/TCB) ──────────────────────
//
// Storage<UTC> holds TAI-seconds-since-J2000-TT plus a leap-label flag. The
// axis conversion is therefore a pure relabeling of the scalar; the leap
// flag is dropped on the TAI side and cleared when entering UTC from TAI
// (the civil layer sets it explicitly when needed).

impl InfallibleConvertible<TAI> for UTC {
    #[inline]
    fn convert(src: Storage<UTC>) -> Storage<TAI> {
        Storage::new_unchecked(src.seconds, false)
    }
}
impl InfallibleConvertible<UTC> for TAI {
    #[inline]
    fn convert(src: Storage<TAI>) -> Storage<UTC> {
        Storage::new_unchecked(src.seconds, false)
    }
}

macro_rules! utc_through_tai {
    ($axis:ty) => {
        impl InfallibleConvertible<$axis> for UTC {
            #[inline]
            fn convert(src: Storage<UTC>) -> Storage<$axis> {
                let tai = <UTC as InfallibleConvertible<TAI>>::convert(src);
                <TAI as InfallibleConvertible<$axis>>::convert(tai)
            }
        }
        impl InfallibleConvertible<UTC> for $axis {
            #[inline]
            fn convert(src: Storage<$axis>) -> Storage<UTC> {
                let tai = <$axis as InfallibleConvertible<TAI>>::convert(src);
                <TAI as InfallibleConvertible<UTC>>::convert(tai)
            }
        }
    };
}
utc_through_tai!(TT);
utc_through_tai!(TDB);
utc_through_tai!(TCG);
utc_through_tai!(TCB);

// ── UT1 ↔ TT (context-required through ΔT) ───────────────────────────────
//
// Storage on UT1 is SI seconds since J2000 TT *counted on the UT1 axis*.
// The conversion uses ΔT = TT − UT1 as a function of the UT1-axis JD.
// Out-of-horizon epochs return `Ut1HorizonExceeded`; within the compiled
// series the quadratic tail-fit bounds uncertainty explicitly.

#[inline]
fn jd_ut1_from_ut1_seconds(seconds: Seconds) -> Days {
    j2000_seconds_to_jd(seconds)
}

/// Guard: the extrapolated ΔT series is defined up to
/// `DELTA_T_PREDICTION_HORIZON_MJD`; beyond that the quadratic tail-fit
/// still produces a value, but we treat it as out-of-horizon so silent
/// extrapolation never leaks through `to_with`.
#[inline]
fn check_ut1_horizon(mjd: Days) -> Result<(), ConversionError> {
    if mjd <= DELTA_T_PREDICTION_HORIZON_MJD {
        Ok(())
    } else {
        Err(ConversionError::Ut1HorizonExceeded)
    }
}

impl ContextConvertible<TT> for UT1 {
    #[inline]
    fn convert_with(src: Storage<UT1>, _ctx: &TimeContext) -> Result<Storage<TT>, ConversionError> {
        if !src.seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        let jd_ut1: Days = jd_ut1_from_ut1_seconds(src.seconds);
        check_ut1_horizon(jd_to_mjd(jd_ut1))?;
        let dt = delta_t_seconds(jd_ut1);
        Ok(Storage::new_unchecked(src.seconds + dt, false))
    }
}

impl ContextConvertible<UT1> for TT {
    #[inline]
    fn convert_with(src: Storage<TT>, _ctx: &TimeContext) -> Result<Storage<UT1>, ConversionError> {
        if !src.seconds.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        // Solve ut1 + ΔT(ut1) = tt by fixed-point iteration.
        // dΔT/dJD ≈ 3e-8, so three iterations are already below ULP.
        let mut ut1_seconds = src.seconds;
        for _ in 0..3 {
            let jd_ut1: Days = jd_ut1_from_ut1_seconds(ut1_seconds);
            let dt = delta_t_seconds(jd_ut1);
            ut1_seconds = src.seconds - dt;
        }
        let final_jd_ut1: Days = jd_ut1_from_ut1_seconds(ut1_seconds);
        check_ut1_horizon(jd_to_mjd(final_jd_ut1))?;
        Ok(Storage::new_unchecked(ut1_seconds, false))
    }
}

// Transitive UT1 ↔ {TAI, TDB, TCG, TCB, UTC} via TT.
macro_rules! ut1_through_tt_infallible {
    ($axis:ty) => {
        impl ContextConvertible<$axis> for UT1 {
            #[inline]
            fn convert_with(
                src: Storage<UT1>,
                ctx: &TimeContext,
            ) -> Result<Storage<$axis>, ConversionError> {
                let tt = <UT1 as ContextConvertible<TT>>::convert_with(src, ctx)?;
                Ok(<TT as InfallibleConvertible<$axis>>::convert(tt))
            }
        }
        impl ContextConvertible<UT1> for $axis {
            #[inline]
            fn convert_with(
                src: Storage<$axis>,
                ctx: &TimeContext,
            ) -> Result<Storage<UT1>, ConversionError> {
                let tt = <$axis as InfallibleConvertible<TT>>::convert(src);
                <TT as ContextConvertible<UT1>>::convert_with(tt, ctx)
            }
        }
    };
}
ut1_through_tt_infallible!(TAI);
ut1_through_tt_infallible!(TDB);
ut1_through_tt_infallible!(TCG);
ut1_through_tt_infallible!(TCB);
ut1_through_tt_infallible!(UTC);
