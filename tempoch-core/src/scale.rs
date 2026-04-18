// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Frozen scale set.
//!
//! Every scale is a zero-sized marker type that identifies a scientifically
//! distinct time axis. The [`Scale`] trait is sealed — downstream crates
//! cannot add new scales.
//!
//! * Continuous scales (`TAI`, `TT`, `TDB`, `TCG`, `TCB`, `UT1`) implement
//!   [`ContinuousScale`] and support direct arithmetic on `Time<S, F>`.
//! * The civil scale `UTC` does **not** implement `ContinuousScale`;
//!   arithmetic is intentionally absent (RFC §9).

use super::sealed::Sealed;

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
    /// Leap-second-aware civil scale. `Time<UTC, F>` stores the value in
    /// format `F`; the leap-second flag is computed on demand from the
    /// UTC-TAI segment table. Direct arithmetic is intentionally absent.
    ///
    /// # Storage invariant
    ///
    /// `Time<UTC, F>` and `Time<TAI, F>` store **the same numerical value**
    /// for the same physical instant: J2000 TT seconds (or the equivalent in
    /// format `F`) on the continuous TAI axis. They are identical
    /// bit-for-bit. Scale conversion between them is therefore a no-op at
    /// the numeric level.
    ///
    /// This means `.si_seconds()` on a `Time<UTC>` returns a **TAI-based**
    /// J2000 TT second count, *not* a UTC offset or a UTC coordinate value.
    /// UTC differs from TAI by up to 37 s (as of 2017) due to leap seconds;
    /// that discontinuous offset exists only in the *civil* interpretation.
    ///
    /// # Authoritative UTC API
    ///
    /// Use the civil layer for any operation that depends on the UTC-TAI
    /// offset (i.e., leap seconds):
    ///
    /// * [`Time::<UTC>::from_chrono`] / [`try_from_chrono`] / [`try_to_chrono`]
    /// * [`Time::<UTC>::from_unix_seconds`] / [`unix_seconds`]
    ///
    /// [`try_from_chrono`]: crate::Time::try_from_chrono
    /// [`try_to_chrono`]: crate::Time::try_to_chrono
    /// [`from_unix_seconds`]: crate::Time::from_unix_seconds
    /// [`unix_seconds`]: crate::Time::unix_seconds
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
    /// finite: the implementation is documented to stay within about 2 µs only
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
    ///   interpolation. Observed points are accurate to ~0.01 s; prediction
    ///   points (beyond `MODERN_DELTA_T_OBSERVED_END_MJD`) carry growing
    ///   uncertainty. See [`DELTA_T_PREDICTION_HORIZON_MJD`] for the hard stop.
    ///
    /// This model is suitable for archival astronomy and telescope scheduling,
    /// but **not** for precision geodesy, VLBI, or pulsar timing, which
    /// require daily IERS EOP (DUT1) solutions. The compiled monthly series
    /// can differ from daily IERS values by up to ~1 s in recent years.
    ///
    /// [`DELTA_T_PREDICTION_HORIZON_MJD`]: crate::DELTA_T_PREDICTION_HORIZON_MJD
    UT1 = "UT1"
);

// ── ContinuousScale witness ──────────────────────────────────────────────

/// Witness that a scale is continuous and supports direct arithmetic.
/// `UTC` deliberately does not implement this (see RFC §9).
///
/// Sealed — downstream cannot implement it.
#[allow(private_bounds)]
pub trait ContinuousScale: Scale + Sealed {}

macro_rules! continuous {
    ($($scale:ty),+ $(,)?) => {
        $(impl ContinuousScale for $scale {})+
    };
}
continuous!(TAI, TT, TDB, TCG, TCB, UT1);
