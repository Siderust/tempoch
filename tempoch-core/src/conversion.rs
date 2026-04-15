// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Conversion matrix.
//!
//! Two disjoint witness traits enumerate which axis pairs are valid:
//!
//! * [`InfallibleConvertible`] — exact/affine, context-free, always succeeds.
//!   Used by `Time::to::<A2>()`.
//! * [`ContextConvertible`]     — requires a `TimeContext` (UT1 routes).
//!   Used by `Time::to_with::<A2>(&ctx)`.
//!
//! Each witness carries the conversion as an associated function so `Time<A>`
//! never pattern-matches on axis at runtime — the compiler inlines the
//! concrete path.  The `Store` associated type on each `Axis` means that
//! UTC↔TAI conversions are structurally typed (UtcStore ↔ ContinuousStore)
//! rather than a runtime relabeling.

use super::axis::{Axis, TAI, TCB, TCG, TDB, TT, UT1, UTC};
use super::constats::{IAU_TIME_EPOCH_T0_JD, L_B, L_G, TDB0, TT_MINUS_TAI};
use super::context::TimeContext;
use super::delta_t::{delta_t_seconds, DELTA_T_PREDICTION_HORIZON_MJD};
use super::encoding::{
    j2000_seconds_to_jd, jd_to_j2000_seconds, jd_to_julian_centuries, jd_to_mjd,
    mjd_to_j2000_seconds,
};
use super::error::ConversionError;
use super::sealed::Sealed;
use super::storage::{ContinuousStore, UtcStore};
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
    mjd_to_j2000_seconds(crate::constats::UNIX_EPOCH_MJD) + ls + TT_MINUS_TAI
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

/// Witness that converting `Self → A2` is an infallible closed-form operation.
///
/// `src` and the return value use the axis's associated `Store` type, so the
/// compiler can verify the structural correctness of each conversion at compile
/// time (e.g. UTC→TAI produces a `ContinuousStore`, not a `UtcStore`).
pub(crate) trait InfallibleConvertible<A2: Axis>: Axis + Sealed {
    fn convert(src: Self::Store) -> A2::Store;
}

/// Witness that converting `Self → A2` requires a `TimeContext` (UT1 routes).
pub(crate) trait ContextConvertible<A2: Axis>: Axis + Sealed {
    fn convert_with(src: Self::Store, ctx: &TimeContext) -> Result<A2::Store, ConversionError>;
}

// ── Identity ──────────────────────────────────────────────────────────────

macro_rules! identity_infallible {
    ($($axis:ty),+ $(,)?) => {
        $(
            impl InfallibleConvertible<$axis> for $axis {
                #[inline]
                fn convert(src: <$axis as Axis>::Store) -> <$axis as Axis>::Store { src }
            }
        )+
    };
}
identity_infallible!(TAI, TT, TDB, TCG, TCB, UTC, UT1);

// ── TAI ↔ TT (exact affine offset) ───────────────────────────────────────

impl InfallibleConvertible<TT> for TAI {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        ContinuousStore(src.0 + TT_MINUS_TAI)
    }
}
impl InfallibleConvertible<TAI> for TT {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        ContinuousStore(src.0 - TT_MINUS_TAI)
    }
}

// ── TT ↔ TDB (Fairhead–Bretagnon) ────────────────────────────────────────

impl InfallibleConvertible<TDB> for TT {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        let jd_tt: Days = j2000_seconds_to_jd(src.0);
        let delta: Seconds = tdb_minus_tt_seconds(jd_tt);
        ContinuousStore(src.0 + delta)
    }
}
impl InfallibleConvertible<TT> for TDB {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        let mut jd_tt: Days = j2000_seconds_to_jd(src.0);
        for _ in 0..2 {
            jd_tt = j2000_seconds_to_jd(src.0 - tdb_minus_tt_seconds(jd_tt));
        }
        let delta: Seconds = tdb_minus_tt_seconds(jd_tt);
        ContinuousStore(src.0 - delta)
    }
}

// ── TT ↔ TCG (IAU 2000 B1.9 linear rate) ─────────────────────────────────

impl InfallibleConvertible<TCG> for TT {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        ContinuousStore(src.0 + L_G * (src.0 - t0) / (1.0 - L_G))
    }
}
impl InfallibleConvertible<TT> for TCG {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        ContinuousStore(src.0 - L_G * (src.0 - t0))
    }
}

// ── TDB ↔ TCB (IAU 2006 B3 linear relation) ──────────────────────────────

impl InfallibleConvertible<TCB> for TDB {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = src.0 - t0 - TDB0;
        ContinuousStore(t0 + delta / (1.0 - L_B))
    }
}
impl InfallibleConvertible<TDB> for TCB {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = src.0 - t0;
        ContinuousStore(t0 + (1.0 - L_B) * delta + TDB0)
    }
}

// ── Transitive continuous pairs through TT ────────────────────────────────

macro_rules! through_tt {
    ($from:ty, $to:ty) => {
        impl InfallibleConvertible<$to> for $from {
            #[inline]
            fn convert(src: ContinuousStore) -> ContinuousStore {
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
    fn convert(src: ContinuousStore) -> ContinuousStore {
        let tdb = <TT as InfallibleConvertible<TDB>>::convert(src);
        <TDB as InfallibleConvertible<TCB>>::convert(tdb)
    }
}
impl InfallibleConvertible<TT> for TCB {
    #[inline]
    fn convert(src: ContinuousStore) -> ContinuousStore {
        let tdb = <TCB as InfallibleConvertible<TDB>>::convert(src);
        <TDB as InfallibleConvertible<TT>>::convert(tdb)
    }
}

// ── UTC ↔ TAI (structural relabeling) ────────────────────────────────────
//
// `UtcStore` holds TAI-equivalent seconds. Converting to TAI is a pure
// structural coercion (drop the leap label); the reverse wraps in UtcStore
// with `leap = false` (the civil layer sets the flag explicitly when needed).

impl InfallibleConvertible<TAI> for UTC {
    #[inline]
    fn convert(src: UtcStore) -> ContinuousStore {
        ContinuousStore(src.seconds)
    }
}
impl InfallibleConvertible<UTC> for TAI {
    #[inline]
    fn convert(src: ContinuousStore) -> UtcStore {
        UtcStore { seconds: src.0, leap: false }
    }
}

// UTC ↔ TT/TDB/TCG/TCB via TAI.
macro_rules! utc_through_tai {
    ($axis:ty) => {
        impl InfallibleConvertible<$axis> for UTC {
            #[inline]
            fn convert(src: UtcStore) -> ContinuousStore {
                let tai = <UTC as InfallibleConvertible<TAI>>::convert(src);
                <TAI as InfallibleConvertible<$axis>>::convert(tai)
            }
        }
        impl InfallibleConvertible<UTC> for $axis {
            #[inline]
            fn convert(src: ContinuousStore) -> UtcStore {
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

impl ContextConvertible<TT> for UT1 {
    #[inline]
    fn convert_with(src: ContinuousStore, _ctx: &TimeContext) -> Result<ContinuousStore, ConversionError> {
        if !src.0.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        let jd_ut1: Days = j2000_seconds_to_jd(src.0);
        check_ut1_horizon(jd_to_mjd(jd_ut1))?;
        let dt = delta_t_seconds(jd_ut1);
        Ok(ContinuousStore(src.0 + dt))
    }
}

impl ContextConvertible<UT1> for TT {
    #[inline]
    fn convert_with(src: ContinuousStore, _ctx: &TimeContext) -> Result<ContinuousStore, ConversionError> {
        if !src.0.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        // Fixed-point iteration: solve ut1 + ΔT(ut1) = tt.
        // dΔT/dJD ≈ 3e-8; three iterations are already below ULP.
        let mut ut1_secs = src.0;
        for _ in 0..3 {
            let jd_ut1: Days = j2000_seconds_to_jd(ut1_secs);
            let dt = delta_t_seconds(jd_ut1);
            ut1_secs = src.0 - dt;
        }
        let final_jd = j2000_seconds_to_jd(ut1_secs);
        check_ut1_horizon(jd_to_mjd(final_jd))?;
        Ok(ContinuousStore(ut1_secs))
    }
}

// Transitive UT1 ↔ {TAI, TDB, TCG, TCB, UTC} via TT.
macro_rules! ut1_through_tt {
    ($axis:ty) => {
        impl ContextConvertible<$axis> for UT1 {
            #[inline]
            fn convert_with(
                src: ContinuousStore,
                ctx: &TimeContext,
            ) -> Result<<$axis as Axis>::Store, ConversionError> {
                let tt: ContinuousStore =
                    <UT1 as ContextConvertible<TT>>::convert_with(src, ctx)?;
                Ok(<TT as InfallibleConvertible<$axis>>::convert(tt))
            }
        }
        impl ContextConvertible<UT1> for $axis {
            #[inline]
            fn convert_with(
                src: <$axis as Axis>::Store,
                ctx: &TimeContext,
            ) -> Result<ContinuousStore, ConversionError> {
                let tt: ContinuousStore =
                    <$axis as InfallibleConvertible<TT>>::convert(src);
                <TT as ContextConvertible<UT1>>::convert_with(tt, ctx)
            }
        }
    };
}
ut1_through_tt!(TAI);
ut1_through_tt!(TDB);
ut1_through_tt!(TCG);
ut1_through_tt!(TCB);
ut1_through_tt!(UTC);

