// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scale conversion matrix.
//!
//! Two disjoint witness traits enumerate which scale pairs are valid:
//!
//! * [`InfallibleScaleConvert`] — exact/affine, context-free, always succeeds.
//!   Used by `Time::to_scale::<S2>()`.
//! * [`ContextScaleConvert`]   — requires a `TimeContext` (UT1 routes).
//!   Used by `Time::to_scale_with::<S2>(&ctx)`.
//!
//! All conversions operate on `Quantity<Second, f64>` (J2000 TT seconds).
//! The format layer handles lifting to/from the canonical representation.

use super::constats::{IAU_TIME_EPOCH_T0_JD, L_B, L_G, TDB0, TT_MINUS_TAI};
use super::context::TimeContext;
use super::delta_t::{delta_t_seconds, DELTA_T_PREDICTION_HORIZON_MJD};
use super::encoding::{j2000_seconds_to_jd, jd_to_j2000_seconds, jd_to_julian_centuries, jd_to_mjd};
use super::error::ConversionError;
use super::scale::{Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
use super::sealed::Sealed;
use crate::generated::time_data::{UtcTaiSegment, UTC_TAI_SEGMENTS};
use crate::generated::{PRE_1961_TAI_MINUS_UTC_APPROX, UTC_TAI_HISTORY_START_MJD};
use qtty::time::{Days, Seconds};
use qtty::unit::Day;

/// Unix epoch (1970-01-01T00:00:00 UTC) expressed as seconds since J2000 TT
/// on the TAI axis. Relies on the compiled UTC-TAI history.
#[allow(dead_code)]
#[inline]
pub(crate) fn unix_epoch_tai_secs() -> Seconds {
    let ls = try_tai_minus_utc_mjd(crate::constats::UNIX_EPOCH_MJD)
        .unwrap_or(PRE_1961_TAI_MINUS_UTC_APPROX);
    super::encoding::mjd_to_j2000_seconds(crate::constats::UNIX_EPOCH_MJD) + ls + TT_MINUS_TAI
}

// ── UTC-TAI history lookup ────────────────────────────────────────────────

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

// ── TDB ↔ TT: truncated Fairhead–Bretagnon (USNO Circular 179 §2.6) ─────────
//
// Three dominant terms, max error < 30 µs:
//   amplitude   angular frequency (rad/cy)   phase (rad)
//   0.001657    628.3076  (Earth mean anomaly)     6.2401
//   0.000022    575.3385  (Earth-Jupiter synodic)  4.7027
//   0.000014   1256.6152  (2× Earth mean anomaly)  6.2401

#[inline]
fn tdb_minus_tt_seconds(jd_tt: Days) -> Seconds {
    let t = jd_to_julian_centuries(jd_tt);
    Seconds::new(
        0.001_657 * (628.3076 * t + 6.2401).sin()
            + 0.000_022 * (575.3385 * t + 4.7027).sin()
            + 0.000_014 * (1256.6152 * t + 6.2401).sin(),
    )
}

// ── Witness traits ────────────────────────────────────────────────────────

/// Witness that converting `Self → S2` is an infallible closed-form operation.
///
/// All conversions operate on J2000 TT seconds (`Quantity<Second, f64>`).
pub(crate) trait InfallibleScaleConvert<S2: Scale>: Scale + Sealed {
    fn convert(src: Seconds) -> Seconds;
}

/// Witness that converting `Self → S2` requires a `TimeContext` (UT1 routes).
pub(crate) trait ContextScaleConvert<S2: Scale>: Scale + Sealed {
    fn convert_with(src: Seconds, ctx: &TimeContext) -> Result<Seconds, ConversionError>;
}

// ── Identity ──────────────────────────────────────────────────────────────

macro_rules! identity_infallible {
    ($($scale:ty),+ $(,)?) => {
        $(
            impl InfallibleScaleConvert<$scale> for $scale {
                #[inline]
                fn convert(src: Seconds) -> Seconds { src }
            }
        )+
    };
}
identity_infallible!(TAI, TT, TDB, TCG, TCB, UTC, UT1);

// ── TAI ↔ TT (exact affine offset) ───────────────────────────────────────

impl InfallibleScaleConvert<TT> for TAI {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        src + TT_MINUS_TAI
    }
}
impl InfallibleScaleConvert<TAI> for TT {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        src - TT_MINUS_TAI
    }
}

// ── TT ↔ TDB (Fairhead–Bretagnon) ────────────────────────────────────────

impl InfallibleScaleConvert<TDB> for TT {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        let jd_tt: Days = j2000_seconds_to_jd(src);
        let delta: Seconds = tdb_minus_tt_seconds(jd_tt);
        src + delta
    }
}
impl InfallibleScaleConvert<TT> for TDB {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        let mut jd_tt: Days = j2000_seconds_to_jd(src);
        for _ in 0..2 {
            jd_tt = j2000_seconds_to_jd(src - tdb_minus_tt_seconds(jd_tt));
        }
        let delta: Seconds = tdb_minus_tt_seconds(jd_tt);
        src - delta
    }
}

// ── TT ↔ TCG (IAU 2000 B1.9 linear rate) ─────────────────────────────────

impl InfallibleScaleConvert<TCG> for TT {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        src + L_G * (src - t0) / (1.0 - L_G)
    }
}
impl InfallibleScaleConvert<TT> for TCG {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        src - L_G * (src - t0)
    }
}

// ── TDB ↔ TCB (IAU 2006 B3 linear relation) ──────────────────────────────

impl InfallibleScaleConvert<TCB> for TDB {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = src - t0 - TDB0;
        t0 + delta / (1.0 - L_B)
    }
}
impl InfallibleScaleConvert<TDB> for TCB {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = src - t0;
        t0 + (1.0 - L_B) * delta + TDB0
    }
}

// ── Transitive continuous pairs through TT ────────────────────────────────

macro_rules! through_tt {
    ($from:ty, $to:ty) => {
        impl InfallibleScaleConvert<$to> for $from {
            #[inline]
            fn convert(src: Seconds) -> Seconds {
                let tt = <$from as InfallibleScaleConvert<TT>>::convert(src);
                <TT as InfallibleScaleConvert<$to>>::convert(tt)
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

impl InfallibleScaleConvert<TCB> for TT {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        let tdb = <TT as InfallibleScaleConvert<TDB>>::convert(src);
        <TDB as InfallibleScaleConvert<TCB>>::convert(tdb)
    }
}
impl InfallibleScaleConvert<TT> for TCB {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        let tdb = <TCB as InfallibleScaleConvert<TDB>>::convert(src);
        <TDB as InfallibleScaleConvert<TT>>::convert(tdb)
    }
}

// ── UTC ↔ TAI (identity in J2000 TT seconds) ────────────────────────────
//
// Both UTC and TAI store the same J2000 TT second value for the same
// physical instant (see the `UTC` scale doc for the full invariant).
// Scale conversion between them is therefore numerically a no-op.
//
// The UTC-TAI offset (leap seconds) is handled exclusively in the civil
// layer (chrono interop, Unix/GPS encoding). It is never applied in these
// scale-level conversions.
//
// Implication for callers: `.si_seconds()` on `Time<UTC>` returns a
// **TAI-based** continuous count, not a UTC offset. Use the civil API
// (`from_unix_seconds`, `from_chrono`, `unix_seconds`, `try_to_chrono`)
// for any leap-second-aware UTC operation.

impl InfallibleScaleConvert<TAI> for UTC {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        src
    }
}
impl InfallibleScaleConvert<UTC> for TAI {
    #[inline]
    fn convert(src: Seconds) -> Seconds {
        src
    }
}

// UTC ↔ TT/TDB/TCG/TCB via TAI.
macro_rules! utc_through_tai {
    ($scale:ty) => {
        impl InfallibleScaleConvert<$scale> for UTC {
            #[inline]
            fn convert(src: Seconds) -> Seconds {
                let tai = <UTC as InfallibleScaleConvert<TAI>>::convert(src);
                <TAI as InfallibleScaleConvert<$scale>>::convert(tai)
            }
        }
        impl InfallibleScaleConvert<UTC> for $scale {
            #[inline]
            fn convert(src: Seconds) -> Seconds {
                let tai = <$scale as InfallibleScaleConvert<TAI>>::convert(src);
                <TAI as InfallibleScaleConvert<UTC>>::convert(tai)
            }
        }
    };
}
utc_through_tai!(TT);
utc_through_tai!(TDB);
utc_through_tai!(TCG);
utc_through_tai!(TCB);

// ── UT1 ↔ TT (context-required, ΔT model) ────────────────────────────────
//
// ΔT = TT − UT1 is a function of the UT1-axis JD. Out-of-horizon epochs
// return `Ut1HorizonExceeded`.

#[inline]
fn check_ut1_horizon(mjd: Days) -> Result<(), ConversionError> {
    if mjd <= DELTA_T_PREDICTION_HORIZON_MJD {
        Ok(())
    } else {
        Err(ConversionError::Ut1HorizonExceeded)
    }
}

impl ContextScaleConvert<TT> for UT1 {
    #[inline]
    fn convert_with(src: Seconds, _ctx: &TimeContext) -> Result<Seconds, ConversionError> {
        if !src.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        let jd_ut1: Days = j2000_seconds_to_jd(src);
        check_ut1_horizon(jd_to_mjd(jd_ut1))?;
        let dt = delta_t_seconds(jd_ut1);
        Ok(src + dt)
    }
}

impl ContextScaleConvert<UT1> for TT {
    #[inline]
    fn convert_with(src: Seconds, _ctx: &TimeContext) -> Result<Seconds, ConversionError> {
        if !src.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        // Fixed-point iteration: solve ut1 + ΔT(ut1) = tt.
        // dΔT/dJD ≈ 3e-8; three iterations are already below ULP.
        let mut ut1_secs = src;
        for _ in 0..3 {
            let jd_ut1: Days = j2000_seconds_to_jd(ut1_secs);
            let dt = delta_t_seconds(jd_ut1);
            ut1_secs = src - dt;
        }
        let final_jd = j2000_seconds_to_jd(ut1_secs);
        check_ut1_horizon(jd_to_mjd(final_jd))?;
        Ok(ut1_secs)
    }
}

// Transitive UT1 ↔ {TAI, TDB, TCG, TCB, UTC} via TT.
macro_rules! ut1_through_tt {
    ($scale:ty) => {
        impl ContextScaleConvert<$scale> for UT1 {
            #[inline]
            fn convert_with(
                src: Seconds,
                ctx: &TimeContext,
            ) -> Result<Seconds, ConversionError> {
                let tt = <UT1 as ContextScaleConvert<TT>>::convert_with(src, ctx)?;
                Ok(<TT as InfallibleScaleConvert<$scale>>::convert(tt))
            }
        }
        impl ContextScaleConvert<UT1> for $scale {
            #[inline]
            fn convert_with(
                src: Seconds,
                ctx: &TimeContext,
            ) -> Result<Seconds, ConversionError> {
                let tt = <$scale as InfallibleScaleConvert<TT>>::convert(src);
                <TT as ContextScaleConvert<UT1>>::convert_with(tt, ctx)
            }
        }
    };
}
ut1_through_tt!(TAI);
ut1_through_tt!(TDB);
ut1_through_tt!(TCG);
ut1_through_tt!(TCB);
ut1_through_tt!(UTC);
