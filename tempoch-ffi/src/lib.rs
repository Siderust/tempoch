// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Vallés Puig, Ramon

//! C FFI bindings for **tempoch** — astronomical time primitives.
//!
//! This crate exposes a flat C-compatible API for creating and manipulating
//! Julian Dates, Modified Julian Dates, time periods, and UTC conversions.

mod error;
mod period;
mod time;

pub use error::*;
pub use period::*;
pub use time::*;

/// Returns the tempoch-ffi ABI version (semver-encoded: major*10000 + minor*100 + patch).
#[allow(clippy::erasing_op, clippy::identity_op)]
#[no_mangle]
pub extern "C" fn tempoch_ffi_version() -> u32 {
    0 * 10000 + 1 * 100 + 0 // 0.1.0
}
