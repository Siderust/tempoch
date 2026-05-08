// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Frozen scale set.
//!
//! Every scale is a zero-sized marker type that identifies a scientifically
//! distinct time axis. The [`Scale`] trait is sealed — downstream crates
//! cannot add new scales.
//!
//! * Coordinate scales (`TAI`, `TT`, `TDB`, `TCG`, `TCB`, `UT1`, `UTC`)
//!   implement [`CoordinateScale`] and support raw-axis constructors,
//!   accessors, and instant arithmetic on `Time<S>`.
//! * The civil scale `UTC` still does **not** implement [`ContinuousScale`]:
//!   it shares the internal instant axis used by `TAI`, but civil labels and
//!   leap-second interpretation remain table-driven.

use super::sealed::Sealed;

pub(crate) mod conversion;

/// Marker trait for a scientifically distinct time scale.
///
/// Sealed: implementations live in this crate only — downstream crates cannot
/// add new scales.
#[allow(private_bounds)]
pub trait Scale: Sealed + Copy + Clone + core::fmt::Debug + 'static {
    /// Display name of the scale. Used by `Debug` on `Time`.
    const NAME: &'static str;
}

// ── Scale macros ─────────────────────────────────────────────────────────

macro_rules! define_continuous_scale {
    ($(#[$meta:meta])* $ident:ident = $name:literal) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ident;
        impl Sealed for $ident {}
        impl Scale for $ident {
            const NAME: &'static str = $name;
        }
    };
}

macro_rules! define_civil_scale {
    ($(#[$meta:meta])* $ident:ident = $name:literal) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ident;
        impl Sealed for $ident {}
        impl Scale for $ident {
            const NAME: &'static str = $name;
        }
    };
}

// ── Scale definitions ────────────────────────────────────────────────────

define_civil_scale!(
    /// Coordinated Universal Time.
    ///
    /// Leap-second-aware civil scale. `Time<UTC>` stores a continuous instant;
    /// leap-second interpretation is computed on demand from the UTC-TAI
    /// segment table. Raw-axis arithmetic acts on that stored instant.
    ///
    /// # Storage invariant
    ///
    /// `Time<UTC>` and `Time<TAI>` store **the same continuous instant** for
    /// the same physical event. Scale conversion between them is therefore a
    /// numeric no-op at the storage layer.
    ///
    /// UTC participates in [`CoordinateScale`], so `Time<UTC>` exposes the
    /// same raw J2000/JD/MJD instant-axis helpers and second-based arithmetic
    /// as the other built-in scales. Those operations act on the stored
    /// continuous instant, not on a leap-second-labelled civil clock.
    ///
    /// UTC still does **not** implement [`ContinuousScale`]. Generic code that
    /// wants a scale with no civil semantics should keep using that stricter
    /// bound; generic code that merely needs a raw coordinate axis can use
    /// [`CoordinateScale`].
    ///
    /// # Authoritative UTC API
    ///
    /// Use the civil layer for any operation that depends on the UTC-TAI
    /// offset (i.e., leap seconds):
    ///
    /// * [`crate::Time::<UTC>::from_chrono`] / [`crate::Time::try_from_chrono`] / [`crate::Time::try_to_chrono`]
    /// * Unix time: `time.try_to::<`[`crate::Unix`]`>()` returns a [`crate::UnixTime`]
    UTC = "UTC"
);

define_continuous_scale!(
    /// International Atomic Time. Continuous SI-second clock.
    TAI = "TAI"
);

define_continuous_scale!(
    /// Terrestrial Time. The dynamical reference scale in this crate.
    ///
    /// Related to TAI by `TT = TAI + 32.184 s` (exact).
    TT = "TT"
);

define_continuous_scale!(
    /// Barycentric Dynamical Time.
    ///
    /// Differs from TT by a modeled periodic term using the seven-term
    /// Fairhead–Bretagnon truncation from USNO Circular 179.
    ///
    /// The built-in approximation is context-free because the model has no
    /// runtime-settable parameters, but its advertised high-accuracy regime is
    /// finite: the implementation is documented to stay within about 10 µs only
    /// over the interval bracketed by
    /// [`TDB_TT_MODEL_HIGH_ACCURACY_START_JD`] and
    /// [`TDB_TT_MODEL_HIGH_ACCURACY_END_JD`] (roughly 1600-01-01 to
    /// 2200-01-01 TT). Outside that interval conversions remain available, but
    /// the crate does not claim microsecond-level scientific accuracy.
    ///
    /// [`TDB_TT_MODEL_HIGH_ACCURACY_START_JD`]: crate::constats::TDB_TT_MODEL_HIGH_ACCURACY_START_JD
    /// [`TDB_TT_MODEL_HIGH_ACCURACY_END_JD`]: crate::constats::TDB_TT_MODEL_HIGH_ACCURACY_END_JD
    TDB = "TDB"
);

define_continuous_scale!(
    /// Geocentric Coordinate Time (IAU 2000 B1.9). Linear rate difference to TT.
    TCG = "TCG"
);

define_continuous_scale!(
    /// Barycentric Coordinate Time (IAU 2006 B3). Linear relation to TDB.
    TCB = "TCB"
);

define_continuous_scale!(
    /// Universal Time 1 — Earth-rotation time axis.
    ///
    /// Continuous in SI seconds, but `UT1 ↔ TT` requires a `TimeContext`
    /// because the mapping depends on the compiled ΔT model (and, in future
    /// phases, observed-ΔT data).
    ///
    /// # Accuracy and modeling limitations
    ///
    /// UT1 conversions are backed by a piecewise ΔT model:
    ///
    /// * **Historical (pre-1973)**: polynomial approximations (Stephenson &
    ///   Houlden 1986; Meeus *Astronomical Algorithms*). Accuracy varies from
    ///   ±15 s (1620–1973) to ±hundreds of seconds (pre-948).
    /// * **Modern (1973 – horizon)**: USNO monthly determinations with linear
    ///   interpolation. The default monthly-ΔT path differs from the bundled daily
    ///   IERS-derived path by less than 15 ms over the compiled observed
    ///   overlap, and by less than 0.2 s over the compiled short-range
    ///   prediction overlap. See
    ///   [`DELTA_T_PREDICTION_HORIZON_MJD`] for the hard stop.
    ///
    /// This model is suitable for archival astronomy and telescope scheduling,
    /// but **not** for precision geodesy, VLBI, or pulsar timing, which
    /// require daily IERS EOP (DUT1) solutions. Use
    /// [`crate::TimeContext::with_builtin_eop`] when you want the most accurate
    /// bundled UT1 route.
    ///
    /// [`DELTA_T_PREDICTION_HORIZON_MJD`]: crate::DELTA_T_PREDICTION_HORIZON_MJD
    UT1 = "UT1"
);

// ── ContinuousScale witness ──────────────────────────────────────────────

/// Witness that a scale is continuous and supports direct arithmetic.
/// `UTC` deliberately does not implement this: it has raw-axis accessors
/// through [`CoordinateScale`], but its civil interpretation remains
/// leap-second-aware and table-driven.
///
/// Sealed — downstream cannot implement it.
#[allow(private_bounds)]
pub trait CoordinateScale: Scale + Sealed {}

macro_rules! coordinate {
    ($($scale:ty),+ $(,)?) => {
        $(impl CoordinateScale for $scale {})+
    };
}
coordinate!(TAI, TT, TDB, TCG, TCB, UT1, UTC);

/// Witness that a scale is both coordinate-bearing and physically continuous.
///
/// Sealed — downstream cannot implement it.
#[allow(private_bounds)]
pub trait ContinuousScale: CoordinateScale + Sealed {}

macro_rules! continuous {
    ($($scale:ty),+ $(,)?) => {
        $(impl ContinuousScale for $scale {})+
    };
}
continuous!(TAI, TT, TDB, TCG, TCB, UT1);
