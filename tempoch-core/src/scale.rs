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
    /// Differs from TT by a modeled periodic term (Fairhead–Bretagnon, <30 µs).
    /// The conversion is context-free because the model has no runtime-settable
    /// parameters.
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
