// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Error types for period and interval construction.

use core::fmt;

/// Error constructing an [`super::Interval`] with invalid bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidIntervalError {
    /// `!(start <= end)` (unordered comparisons — includes NaN endpoints).
    StartAfterEnd,
}

impl fmt::Display for InvalidIntervalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("interval start must not be after end")
    }
}

impl std::error::Error for InvalidIntervalError {}

/// Invariants on a period list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodListError {
    /// Interval at `index` has `start > end`.
    InvalidInterval { index: usize },
    /// Interval at `index` is not sorted by start time.
    Unsorted { index: usize },
    /// Interval at `index` overlaps its predecessor.
    Overlapping { index: usize },
}

impl fmt::Display for PeriodListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInterval { index } => {
                write!(f, "interval at index {index} has start > end")
            }
            Self::Unsorted { index } => {
                write!(f, "interval at index {index} is not sorted by start time")
            }
            Self::Overlapping { index } => {
                write!(f, "interval at index {index} overlaps its predecessor")
            }
        }
    }
}

impl std::error::Error for PeriodListError {}
