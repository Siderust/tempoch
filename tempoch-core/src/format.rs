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
//! | [`JD`]       | `Quantity<Day, f64>`       | Julian Day number                 |
//! | [`MJD`]      | `Quantity<Day, f64>`       | Modified Julian Day               |
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

#[cfg(feature = "serde")]
const NONFINITE_TIME_VALUE_ERROR: &str = "time value must be finite (not NaN or infinity)";

#[cfg(feature = "serde")]
#[allow(private_bounds)]
pub(crate) trait SerdeFormat: Format {
    fn validate_serde_value(value: &Self::Storage) -> Result<(), &'static str>;
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
    JD,
    Quantity<Day, f64>,
    "JD"
);

define_format!(
    /// Modified Julian Day (JD − 2 400 000.5).
    ///
    /// Storage: `Quantity<Day, f64>`.
    MJD,
    Quantity<Day, f64>,
    "MJD"
);

define_format!(
    /// Unix/POSIX seconds since 1970-01-01T00:00:00 UTC.
    ///
    /// Storage: `Quantity<Second, i64>` — natural integer representation.
    ///
    /// # Civil API — no generic `reformat()` path
    ///
    /// `UnixSecs` deliberately has **no** `FormatConvertible` implementations
    /// to other formats.  A naïve epoch-offset conversion would produce wrong
    /// POSIX timestamps for all dates: the correct mapping requires subtracting
    /// the history-dependent `TAI−UTC` offset, which is not available in the
    /// format layer.
    ///
    /// **Authoritative POSIX mapping:** use the civil API:
    /// [`Time::<UTC>::from_unix_seconds`] / [`.unix_seconds()`].
    ///
    /// [`Time::<UTC>::from_unix_seconds`]: crate::Time::from_unix_seconds
    /// [`.unix_seconds()`]: crate::Time::unix_seconds
    UnixSecs,
    QuantityI64<Second>,
    "Unix"
);

define_format!(
    /// GPS seconds since the GPS epoch (1980-01-06T00:00:00 UTC).
    ///
    /// Storage: `Quantity<Second, f64>`.
    ///
    /// # Scale semantics warning
    ///
    /// GPS time runs at exactly the TAI rate with a fixed 19-second offset.
    /// The `GpsSecs` format's epoch constant (`GPS_EPOCH_TAI`) is calibrated
    /// on the **TAI axis**. Using this format on any other scale (TT, TDB,
    /// UTC, …) will store a value that is *not* a valid GPS timestamp — it
    /// will differ from true GPS by `TT−TAI = 32.184 s` (for `Time<TT>`),
    /// `TDB−TT` (for `Time<TDB>`), etc.
    ///
    /// **Authoritative GPS mapping:** use the civil API:
    /// [`Time::<TAI>::from_gps_seconds`] / [`.gps_seconds()`].
    ///
    /// Format-only conversions (`.reformat::<GpsSecs>()` from another format
    /// *on the same scale*) are available, but scale conversions
    /// (`to_scale::<S2>()`) require an explicit `.reformat::<J2000s>()`
    /// first, making the scale-semantic trade-off visible at the call site.
    ///
    /// [`Time::<TAI>::from_gps_seconds`]: crate::Time::from_gps_seconds
    /// [`.gps_seconds()`]: crate::Time::gps_seconds
    GpsSecs,
    Quantity<Second, f64>,
    "GPS"
);

define_format!(
    /// Integer day count (e.g. the integer part of MJD).
    ///
    /// Storage: `Quantity<Day, i32>` — useful for coarse day-level indexing.
    /// Scale conversions are not directly available on this format;
    /// use `.reformat::<MJD>()` or `.reformat::<J2000s>()` first.
    DayCount,
    QuantityI32<Day>,
    "DayCount"
);

#[cfg(feature = "serde")]
macro_rules! impl_serde_format_finite {
    ($($format:ty),+ $(,)?) => {
        $(
            impl SerdeFormat for $format {
                #[inline]
                fn validate_serde_value(value: &Self::Storage) -> Result<(), &'static str> {
                    if value.is_finite() {
                        Ok(())
                    } else {
                        Err(NONFINITE_TIME_VALUE_ERROR)
                    }
                }
            }
        )+
    };
}

#[cfg(feature = "serde")]
macro_rules! impl_serde_format_passthrough {
    ($($format:ty),+ $(,)?) => {
        $(
            impl SerdeFormat for $format {
                #[inline]
                fn validate_serde_value(_value: &Self::Storage) -> Result<(), &'static str> {
                    Ok(())
                }
            }
        )+
    };
}

#[cfg(feature = "serde")]
impl_serde_format_finite!(J2000s, JD, MJD, GpsSecs);

#[cfg(feature = "serde")]
impl_serde_format_passthrough!(UnixSecs, DayCount);
