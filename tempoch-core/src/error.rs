// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

/// Conversion error surface.
///
/// Variants are payload-free in v1 to keep the matrix small; they may carry
/// context in later phases if a concrete call-site demands it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionError {
    /// The UTC instant is before the start of the compiled UTC–TAI history
    /// (1961-01-01).
    UtcHistoryUnsupported,
    /// A leap-second label does not correspond to a leap second in the
    /// compiled UTC history.
    InvalidLeapSecond,
    /// The converted value leaves the representable range of the target.
    OutOfRange,
    /// A UT1 conversion was requested outside the horizon of the configured
    /// ΔT model or observed-data source.
    Ut1HorizonExceeded,
    /// Input or arithmetic produced `NaN` or `±∞`.
    NonFinite,
}

impl core::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UtcHistoryUnsupported => {
                f.write_str("exact UTC conversions are only supported from 1961-01-01 onward")
            }
            Self::InvalidLeapSecond => {
                f.write_str("leap-second label is not present in the compiled UTC history")
            }
            Self::OutOfRange => f.write_str("converted value is out of representable range"),
            Self::Ut1HorizonExceeded => {
                f.write_str("UT1 conversion exceeds the ΔT model or data horizon")
            }
            Self::NonFinite => f.write_str("time value must be finite (not NaN or infinity)"),
        }
    }
}

impl std::error::Error for ConversionError {}
