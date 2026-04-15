// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Private storage layer.
//!
//! Two concrete types encode the structural difference between continuous
//! axes and the civil UTC axis at zero cost:
//!
//! * [`ContinuousStore`] — `#[repr(transparent)]` over `Seconds` (8 bytes).
//!   All continuous-axis `Time<A>` values have the same ABI as a bare `f64`.
//! * [`UtcStore`]        — holds TAI-equivalent seconds plus a leap label.
//!   Used exclusively by the `UTC` axis.
//!
//! The [`AxisStore`] sealed trait provides uniform read access; every method
//! is `#[inline]` and monomorphises to zero-overhead code.

use super::error::ConversionError;
use super::sealed::Sealed;
use qtty::time::Seconds;

// ── AxisStore ─────────────────────────────────────────────────────────────

/// Sealed interface for the two concrete storage variants.
///
/// Downstream crates cannot implement this trait.
#[allow(private_bounds)]
#[doc(hidden)]
pub trait AxisStore: Copy + Clone + core::fmt::Debug + 'static + Sealed {
    fn seconds(self) -> Seconds;
    fn new(seconds: Seconds) -> Result<Self, ConversionError>
    where
        Self: Sized;
    /// Unchecked constructor. Caller must guarantee `seconds.is_finite()`.
    fn new_unchecked(seconds: Seconds, leap: bool) -> Self;
    fn leap(self) -> bool;
}

// ── ContinuousStore ───────────────────────────────────────────────────────

/// Zero-overhead storage for continuous time axes.
///
/// `#[repr(transparent)]` over `Seconds`, so `Time<TT>` (and every other
/// continuous-axis instant) has the same ABI and size as a bare `f64`.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
#[doc(hidden)]
pub struct ContinuousStore(pub(crate) Seconds);

impl Sealed for ContinuousStore {}

impl AxisStore for ContinuousStore {
    #[inline]
    fn seconds(self) -> Seconds {
        self.0
    }

    #[inline]
    fn new(s: Seconds) -> Result<Self, ConversionError> {
        if s.is_finite() {
            Ok(Self(s))
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    #[inline]
    fn new_unchecked(s: Seconds, _leap: bool) -> Self {
        Self(s)
    }

    #[inline]
    fn leap(self) -> bool {
        false
    }
}

// ── UtcStore ──────────────────────────────────────────────────────────────

/// Civil-time storage for the `UTC` axis.
///
/// Holds TAI-equivalent seconds (so the scalar remains continuous across
/// leap seconds) plus a boolean leap-second label.
#[derive(Debug, Copy, Clone)]
#[doc(hidden)]
pub struct UtcStore {
    pub(crate) seconds: Seconds,
    pub(crate) leap: bool,
}

impl Sealed for UtcStore {}

impl AxisStore for UtcStore {
    #[inline]
    fn seconds(self) -> Seconds {
        self.seconds
    }

    #[inline]
    fn new(s: Seconds) -> Result<Self, ConversionError> {
        if s.is_finite() {
            Ok(Self { seconds: s, leap: false })
        } else {
            Err(ConversionError::NonFinite)
        }
    }

    #[inline]
    fn new_unchecked(s: Seconds, leap: bool) -> Self {
        Self { seconds: s, leap }
    }

    #[inline]
    fn leap(self) -> bool {
        self.leap
    }
}
