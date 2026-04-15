// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Frozen axis set.

use super::sealed::Sealed;

/// Marker trait for a scientifically distinct time axis.
///
/// Sealed: implementations live in this crate (and, in a later phase, in
/// `tempoch-astro`) — downstream crates cannot add new axes.
pub trait Axis: Sealed + Copy + Clone + core::fmt::Debug + 'static {
    /// Display name of the axis. Used by `Debug` / `Display` on `Time`.
    const NAME: &'static str;
}

macro_rules! define_axis {
    ($(#[$meta:meta])* $ident:ident = $name:literal) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ident;
        impl Sealed for $ident {}
        impl Axis for $ident {
            const NAME: &'static str = $name;
        }
    };
}

define_axis!(
    /// Coordinated Universal Time.
    ///
    /// First-class axis: leap-second-aware, discontinuous. `Native` on UTC is
    /// an exact civil instant (integer seconds + nanos + leap flag). Arithmetic
    /// via `qtty::Seconds` routes through TAI.
    UTC = "UTC"
);

define_axis!(
    /// International Atomic Time. Continuous SI-second clock.
    TAI = "TAI"
);

define_axis!(
    /// Terrestrial Time. The dynamical reference axis in this crate.
    ///
    /// Related to TAI by `TT = TAI + 32.184 s` (exact).
    TT = "TT"
);

define_axis!(
    /// Barycentric Dynamical Time.
    ///
    /// Differs from TT by a modeled periodic term (Fairhead–Bretagnon, <30 µs).
    /// The conversion is context-free because the model has no runtime-settable
    /// parameters.
    TDB = "TDB"
);

define_axis!(
    /// Geocentric Coordinate Time (IAU 2000 B1.9). Linear rate difference to TT.
    TCG = "TCG"
);

define_axis!(
    /// Barycentric Coordinate Time (IAU 2006 B3). Linear relation to TDB.
    TCB = "TCB"
);

define_axis!(
    /// Universal Time 1 — Earth-rotation time axis.
    ///
    /// Continuous in SI seconds, but `UT1 ↔ TT` requires a `TimeContext`
    /// because the mapping depends on the compiled ΔT model (and, in future
    /// phases, observed-ΔT data).
    UT1 = "UT1"
);
