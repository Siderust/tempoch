// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Representations (RFC 0001 §6).
//!
//! A representation is *how* an instant on an axis is encoded. Valid
//! `(Axis, Representation)` pairs are enumerated here via trait impls; any
//! pair not listed will fail to compile.

use super::axis::{Axis, TAI, TCB, TCG, TDB, TT, UT1, UTC};
use super::sealed::Sealed;
use core::marker::PhantomData;

/// Marker trait for a representation valid on axis `A`.
///
/// Sealed. See module-level docs on `super` for the full list.
pub trait Representation<A: Axis>: Sealed + Copy + Clone + core::fmt::Debug + 'static {
    /// Display name of the representation.
    const NAME: &'static str;
}

/// Canonical default representation for an axis.
///
/// `Native` does not imply a storage layout; storage is private. It denotes
/// the canonical input/output unit per axis (see RFC 0001 §6.2).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Native;
impl Sealed for Native {}

/// Julian Days on the axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JulianDays;
impl Sealed for JulianDays {}

/// Modified Julian Days (`JD − 2_400_000.5`) on the axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModifiedJulianDays;
impl Sealed for ModifiedJulianDays {}

/// SI seconds since J2000 TT, counted on the target axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SISeconds;
impl Sealed for SISeconds {}

/// Convention tag for Unix-seconds representations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct POSIX;
impl Sealed for POSIX {}

/// Unix seconds under a convention tag (currently only `POSIX`).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixSeconds<C: Sealed + Copy + Clone + core::fmt::Debug + 'static>(
    pub(crate) PhantomData<C>,
);
impl<C: Sealed + Copy + Clone + core::fmt::Debug + 'static> Sealed for UnixSeconds<C> {}

/// GPS seconds since the GPS epoch (1980-01-06T00:00:00 UTC).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GpsSeconds;
impl Sealed for GpsSeconds {}

// ── Validity table ────────────────────────────────────────────────────────

// `Native` is valid on every axis. Since both `Native` and `Axis` are
// sealed, this blanket impl is closed — no downstream can expand it.
impl<A: Axis> Representation<A> for Native {
    const NAME: &'static str = "Native";
}

// JulianDays / ModifiedJulianDays / SISeconds: continuous axes only.
// UTC is intentionally excluded — see RFC §6.1.
macro_rules! continuous_on {
    ($repr:ty, $name:literal, $($axis:ty),+ $(,)?) => {
        $(
            impl Representation<$axis> for $repr {
                const NAME: &'static str = $name;
            }
        )+
    };
}
continuous_on!(JulianDays, "JulianDays", TAI, TT, TDB, TCG, TCB, UT1);
continuous_on!(
    ModifiedJulianDays,
    "ModifiedJulianDays",
    TAI,
    TT,
    TDB,
    TCG,
    TCB,
    UT1
);
continuous_on!(SISeconds, "SISeconds", TAI, TT, TDB, TCG, TCB, UT1);

// UnixSeconds<POSIX>: UTC only.
impl Representation<UTC> for UnixSeconds<POSIX> {
    const NAME: &'static str = "UnixSeconds<POSIX>";
}

// GpsSeconds: TAI only.
impl Representation<TAI> for GpsSeconds {
    const NAME: &'static str = "GpsSeconds";
}
