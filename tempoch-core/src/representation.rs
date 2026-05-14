// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Compatibility re-exports from [`crate::format`].
//!
//! This module is kept for crate-internal use only. Public API is provided
//! through the crate root. The canonical module is [`crate::format`].

#[doc(hidden)]
pub use crate::format::{
    EncodedTime, FormatForScale as RepresentationForScale, GpsTime,
    InfallibleFormatForScale as InfallibleRepresentationForScale, J2000Seconds, J2000s, JulianDate,
    MJD, ModifiedJulianDate, TimeFormat as TimeRepresentation, Unix, UnixTime, GPS, JD,
};
