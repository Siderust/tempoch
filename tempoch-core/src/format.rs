// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Time representation formats.
//!
//! A [`Format`] determines the numerical representation and storage type
//! of a [`Time<S, F>`](super::time::Time) value. Each format marker carries
//! an associated `Storage` type backed by a `qtty::Quantity<Unit, Scalar>`,
//! matching the natural datatype for that representation.
//!
//! | Format       | Storage                    | Description                      |
//! |--------------|----------------------------|----------------------------------|
//! | [`J2000s`]   | `Quantity<Second, f64>`    | SI seconds since J2000 TT        |
//! | [`Jd`]       | `Quantity<Day, f64>`       | Julian Day number                 |
//! | [`Mjd`]      | `Quantity<Day, f64>`       | Modified Julian Day               |
//! | [`UnixSecs`] | `Quantity<Second, i64>`    | Seconds since 1970-01-01 UTC      |
//! | [`GpsSecs`]  | `Quantity<Second, f64>`    | Seconds since GPS epoch           |
//! | [`DayCount`] | `Quantity<Day, i32>`       | Integer day count (e.g. MJD int)  |

use super::sealed::Sealed;
use qtty::unit::{Day, Second};
use qtty::{Quantity, QuantityI32, QuantityI64};

/// Marker trait for a time representation format.
///
/// Sealed: implementations live in this crate only — downstream crates cannot
/// add new formats.
///
/// The associated `Storage` type determines what `qtty::Quantity` variant
/// backs a `Time<S, F>` value.
#[allow(private_bounds)]
pub trait Format: Sealed + Copy + Clone + core::fmt::Debug + 'static {
    /// The `qtty::Quantity` type used to store time values in this format.
    type Storage: Copy + Clone + core::fmt::Debug;
    /// Display name of the format. Used by `Debug` on `Time`.
    const NAME: &'static str;
}

// ── Format macros ────────────────────────────────────────────────────────

macro_rules! define_format {
    ($(#[$meta:meta])* $ident:ident, $storage:ty, $name:literal) => {
        $(#[$meta])*
        #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $ident;
        impl Sealed for $ident {}
        impl Format for $ident {
            type Storage = $storage;
            const NAME: &'static str = $name;
        }
    };
}

// ── Format definitions ───────────────────────────────────────────────────

define_format!(
    /// SI seconds since J2000 TT epoch.
    ///
    /// This is the canonical internal format used for scale conversions.
    /// Storage: `Quantity<Second, f64>`.
    J2000s,
    Quantity<Second, f64>,
    "J2000s"
);

define_format!(
    /// Julian Day number (absolute day count from the Julian epoch).
    ///
    /// Storage: `Quantity<Day, f64>`.
    Jd,
    Quantity<Day, f64>,
    "JD"
);

define_format!(
    /// Modified Julian Day (JD − 2 400 000.5).
    ///
    /// Storage: `Quantity<Day, f64>`.
    Mjd,
    Quantity<Day, f64>,
    "MJD"
);

define_format!(
    /// Unix/POSIX seconds since 1970-01-01T00:00:00 UTC.
    ///
    /// Storage: `Quantity<Second, i64>` — natural integer representation.
    /// Scale conversions are not directly available on this format;
    /// use `.reformat::<J2000s>()` first to make the precision trade-off
    /// explicit.
    UnixSecs,
    QuantityI64<Second>,
    "Unix"
);

define_format!(
    /// GPS seconds since the GPS epoch (1980-01-06T00:00:00 UTC).
    ///
    /// Storage: `Quantity<Second, f64>`.
    GpsSecs,
    Quantity<Second, f64>,
    "GPS"
);

define_format!(
    /// Integer day count (e.g. the integer part of MJD).
    ///
    /// Storage: `Quantity<Day, i32>` — useful for coarse day-level indexing.
    /// Scale conversions are not directly available on this format;
    /// use `.reformat::<Mjd>()` or `.reformat::<J2000s>()` first.
    DayCount,
    QuantityI32<Day>,
    "DayCount"
);
