// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! C FFI bindings for **tempoch** — astronomical time primitives.
//!
//! This crate exposes the split, scale/format-aware C ABI for creating and
//! manipulating typed time instants, periods, and UTC conversions.
//!
//! # ABI conventions
//!
//! - Split time instants cross the ABI as [`TempochTime`] `(hi, lo)` pairs.
//! - UTC calendar fields remain in `TempochUtc` as raw integer fields.
//! - Duration-related functions return [`QttyQuantity`] from qtty-ffi.
//! - Generic conversions use explicit scale and format tags validated at the
//!   boundary.
//! - Every fallible function returns `TempochStatus`; `InternalPanic` is
//!   reserved exclusively for caught Rust panics.

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod constats;
mod context;
mod eop;
mod error;
mod period;
mod time;
mod typed;

pub use constats::*;
pub use context::*;
pub use eop::*;
pub use error::*;
pub use period::*;
pub use time::*;
pub use typed::*;

// Re-export qtty-ffi types used in our public FFI surface
pub use qtty_ffi::{QttyQuantity, UnitId};

/// Catches any panic and returns an error value instead of unwinding across FFI.
/// For functions returning `TempochStatus`, the fallback is `InternalPanic`.
macro_rules! catch_panic {
    ($default:expr, $body:expr) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(result) => result,
            Err(_) => $default,
        }
    }};
}
pub(crate) use catch_panic;

/// Returns the tempoch-ffi ABI version (major*10000 + minor*100 + patch).
///
/// Current ABI line: 0.6.x -> 600
#[allow(clippy::erasing_op, clippy::identity_op)]
#[no_mangle]
pub extern "C" fn tempoch_ffi_version() -> u32 {
    0 * 10000 + 6 * 100 + 0 // 0.6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_returns_expected_value() {
        assert_eq!(tempoch_ffi_version(), 600);
    }

    // ── Layout tests ──────────────────────────────────────────────────────
    // These tests guard ABI stability by asserting exact sizes and alignments
    // of every exported `#[repr(C)]` type.

    #[test]
    fn layout_tempoch_status() {
        assert_eq!(std::mem::size_of::<TempochStatus>(), 4);
        assert_eq!(std::mem::align_of::<TempochStatus>(), 4);
    }

    #[test]
    fn layout_tempoch_scale_tag() {
        assert_eq!(std::mem::size_of::<TempochScaleTag>(), 4);
        assert_eq!(std::mem::align_of::<TempochScaleTag>(), 4);
    }

    #[test]
    fn layout_tempoch_format_tag() {
        assert_eq!(std::mem::size_of::<TempochFormatTag>(), 4);
        assert_eq!(std::mem::align_of::<TempochFormatTag>(), 4);
    }

    #[test]
    fn layout_tempoch_utc() {
        // i32 + u8*5 + padding + u32 = 16 with C repr packing
        assert_eq!(std::mem::size_of::<TempochUtc>(), 16);
        assert_eq!(std::mem::align_of::<TempochUtc>(), 4);
    }

    #[test]
    fn layout_tempoch_period_mjd() {
        assert_eq!(std::mem::size_of::<TempochPeriodMjd>(), 16);
        assert_eq!(std::mem::align_of::<TempochPeriodMjd>(), 8);
    }

    #[test]
    fn layout_tempoch_eop_values() {
        assert_eq!(std::mem::size_of::<TempochEopValues>(), 64);
        assert_eq!(std::mem::align_of::<TempochEopValues>(), 8);
    }

    #[test]
    fn layout_tempoch_time() {
        assert_eq!(std::mem::size_of::<TempochTime>(), 16);
        assert_eq!(std::mem::align_of::<TempochTime>(), 8);
    }

    // ── Status discriminants are stable ───────────────────────────────────

    #[test]
    fn status_discriminants_are_stable() {
        assert_eq!(TempochStatus::Ok as i32, 0);
        assert_eq!(TempochStatus::NullPointer as i32, 1);
        assert_eq!(TempochStatus::UtcConversionFailed as i32, 2);
        assert_eq!(TempochStatus::InvalidPeriod as i32, 3);
        assert_eq!(TempochStatus::NoIntersection as i32, 4);
        assert_eq!(TempochStatus::InvalidScaleId as i32, 5);
        assert_eq!(TempochStatus::InvalidDurationUnit as i32, 6);
        assert_eq!(TempochStatus::InternalPanic as i32, 7);
        assert_eq!(TempochStatus::Ut1HorizonExceeded as i32, 8);
        assert_eq!(TempochStatus::PeriodListUnsorted as i32, 9);
        assert_eq!(TempochStatus::PeriodListOverlapping as i32, 10);
        assert_eq!(TempochStatus::ConversionFailed as i32, 11);
        assert_eq!(TempochStatus::InvalidFormatId as i32, 12);
    }

    // ── Split ABI discriminants are stable ────────────────────────────────

    #[test]
    fn scale_tag_discriminants_are_stable() {
        assert_eq!(TempochScaleTag::TT as i32, 0);
        assert_eq!(TempochScaleTag::TAI as i32, 1);
        assert_eq!(TempochScaleTag::UTC as i32, 2);
        assert_eq!(TempochScaleTag::UT1 as i32, 3);
        assert_eq!(TempochScaleTag::TDB as i32, 4);
        assert_eq!(TempochScaleTag::TCG as i32, 5);
        assert_eq!(TempochScaleTag::TCB as i32, 6);
    }

    #[test]
    fn format_tag_discriminants_are_stable() {
        assert_eq!(TempochFormatTag::JD as i32, 0);
        assert_eq!(TempochFormatTag::MJD as i32, 1);
        assert_eq!(TempochFormatTag::J2000Seconds as i32, 2);
        assert_eq!(TempochFormatTag::Unix as i32, 3);
        assert_eq!(TempochFormatTag::GPS as i32, 4);
    }
}
