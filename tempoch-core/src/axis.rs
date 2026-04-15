// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Frozen axis set.
//!
//! Every axis carries an associated [`Axis::Store`] type that encodes the
//! storage layout at the type level:
//!
//! * Continuous axes (`TAI`, `TT`, `TDB`, `TCG`, `TCB`, `UT1`) use
//!   [`ContinuousStore`], which is `#[repr(transparent)]` over `Seconds`.
//!   A `Time<TT>` therefore has the same ABI as a bare `f64`.
//! * The civil axis `UTC` uses [`UtcStore`], which adds a leap-second label.

use super::sealed::Sealed;
use super::storage::{AxisStore, ContinuousStore, UtcStore};

/// Marker trait for a scientifically distinct time axis.
///
/// Sealed: implementations live in this crate only — downstream crates cannot
/// add new axes.
///
/// The associated `Store` type encodes the storage layout for this axis and
/// is the mechanism through which `Time<A>` achieves zero-overhead layout for
/// continuous axes.
#[allow(private_bounds)]
pub trait Axis: Sealed + Copy + Clone + core::fmt::Debug + 'static {
    /// Storage variant used by `Time<Self>`.
    type Store: AxisStore;
    /// Display name of the axis. Used by `Debug` on `Time`.
    const NAME: &'static str;
}

// ── Axis macros ───────────────────────────────────────────────────────────

macro_rules! define_continuous_axis {
    ($(#[$meta:meta])* $ident:ident = $name:literal) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ident;
        impl Sealed for $ident {}
        impl Axis for $ident {
            type Store = ContinuousStore;
            const NAME: &'static str = $name;
        }
    };
}

macro_rules! define_civil_axis {
    ($(#[$meta:meta])* $ident:ident = $name:literal) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ident;
        impl Sealed for $ident {}
        impl Axis for $ident {
            type Store = UtcStore;
            const NAME: &'static str = $name;
        }
    };
}

// ── Axis definitions ──────────────────────────────────────────────────────

define_civil_axis!(
    /// Coordinated Universal Time.
    ///
    /// Leap-second-aware civil axis. Internally `Time<UTC>` stores TAI seconds
    /// (keeping the scalar continuous across leap seconds) plus a leap-second
    /// label. Civil conversions go through `try_to_chrono` / `from_chrono`;
    /// direct SI-second arithmetic is intentionally not available.
    UTC = "UTC"
);

define_continuous_axis!(
    /// International Atomic Time. Continuous SI-second clock.
    TAI = "TAI"
);

define_continuous_axis!(
    /// Terrestrial Time. The dynamical reference axis in this crate.
    ///
    /// Related to TAI by `TT = TAI + 32.184 s` (exact).
    TT = "TT"
);

define_continuous_axis!(
    /// Barycentric Dynamical Time.
    ///
    /// Differs from TT by a modeled periodic term (Fairhead–Bretagnon, <30 µs).
    /// The conversion is context-free because the model has no runtime-settable
    /// parameters.
    TDB = "TDB"
);

define_continuous_axis!(
    /// Geocentric Coordinate Time (IAU 2000 B1.9). Linear rate difference to TT.
    TCG = "TCG"
);

define_continuous_axis!(
    /// Barycentric Coordinate Time (IAU 2006 B3). Linear relation to TDB.
    TCB = "TCB"
);

define_continuous_axis!(
    /// Universal Time 1 — Earth-rotation time axis.
    ///
    /// Continuous in SI seconds, but `UT1 ↔ TT` requires a `TimeContext`
    /// because the mapping depends on the compiled ΔT model (and, in future
    /// phases, observed-ΔT data).
    UT1 = "UT1"
);

// ── ContinuousAxis witness ────────────────────────────────────────────────

/// Witness that an axis uses [`ContinuousStore`] and supports direct
/// `qtty::Second` arithmetic. `UTC` deliberately does not implement this
/// (see RFC §9).
///
/// Sealed — downstream cannot implement it.
#[allow(private_bounds)]
pub trait ContinuousAxis: Axis<Store = ContinuousStore> + Sealed {}

macro_rules! continuous {
    ($($axis:ty),+ $(,)?) => {
        $(impl ContinuousAxis for $axis {})+
    };
}
continuous!(TAI, TT, TDB, TCG, TCB, UT1);
