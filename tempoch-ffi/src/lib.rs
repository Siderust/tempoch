// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! C FFI bindings for **tempoch** — astronomical time primitives.
//!
//! This crate exposes a flat C-compatible API for creating and manipulating
//! Julian Dates, Modified Julian Dates, time periods, and UTC conversions.
//!
//! Duration-related functions return [`QttyQuantity`] from qtty-ffi, providing
//! type-safe unit information alongside numeric values.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod error;
mod period;
mod time;

pub use error::*;
pub use period::*;
pub use time::*;

// Re-export qtty-ffi types used in our public FFI surface
pub use qtty_ffi::{QttyQuantity, UnitId};

/// Catches any panic and returns an error value instead of unwinding across FFI.
macro_rules! catch_panic {
    ($default:expr, $body:expr) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => $default,
        }
    }};
}
pub(crate) use catch_panic;

/// Returns the tempoch-ffi ABI version (semver-encoded: major*10000 + minor*100 + patch).
#[allow(clippy::erasing_op, clippy::identity_op)]
#[no_mangle]
pub extern "C" fn tempoch_ffi_version() -> u32 {
    0 * 10000 + 2 * 100 + 0 // 0.2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_returns_expected_value() {
        assert_eq!(tempoch_ffi_version(), 200);
    }

    // ── Layout tests ─────────────────────────────────────────────────
    // These tests guard ABI stability by asserting exact sizes and
    // alignments of every `#[repr(C)]`/`#[repr(i32)]` type.

    #[test]
    fn layout_tempoch_status() {
        assert_eq!(std::mem::size_of::<TempochStatus>(), 4);
        assert_eq!(std::mem::align_of::<TempochStatus>(), 4);
    }

    #[test]
    fn layout_tempoch_scale() {
        assert_eq!(std::mem::size_of::<TempochScale>(), 4);
        assert_eq!(std::mem::align_of::<TempochScale>(), 4);
    }

    #[test]
    fn layout_tempoch_utc() {
        // i32 + u8*5 + padding + u32 = 16 with C repr packing
        assert_eq!(std::mem::size_of::<TempochUtc>(), 16);
        assert_eq!(std::mem::align_of::<TempochUtc>(), 4);
    }

    #[test]
    fn layout_tempoch_period_mjd() {
        // Two f64 fields → 16 bytes, 8-aligned
        assert_eq!(std::mem::size_of::<TempochPeriodMjd>(), 16);
        assert_eq!(std::mem::align_of::<TempochPeriodMjd>(), 8);
    }
}
