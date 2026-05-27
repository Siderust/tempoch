// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! [`TimeSeries`] — exact-step iterator over `Time<S>`.
//!
//! Generates a uniform sequence of typed instants over a half-open range
//! `[start, end)` with an [`crate::ExactDuration`] step. The step is exact in
//! nanoseconds; the produced `Time<S>` values inherit the split-f64 storage of
//! `Time<S>` (see `foundation::duration` for the W1 caveat that exactness
//! lives in the duration container, not in instant storage).
//!
//! # Examples
//!
//! ```
//! use tempoch_core::{ExactDuration, Time, TimeSeries, TT};
//! use qtty::Second;
//!
//! let start = Time::<TT>::from_raw_j2000_seconds(Second::new(0.0)).unwrap();
//! let end = Time::<TT>::from_raw_j2000_seconds(Second::new(10.0)).unwrap();
//! let series = TimeSeries::new(start, end, ExactDuration::SECOND).unwrap();
//! assert_eq!(series.count(), 10);
//! ```

use crate::format::TimeFormat;
use crate::foundation::duration::{DurationError, ExactDuration};
use crate::model::scale::CoordinateScale;
use crate::model::time::Time;

/// Error returned when a [`TimeSeries`] cannot be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSeriesError {
    /// Step was zero — the iterator would not terminate.
    ZeroStep,
    /// `end < start` — the half-open range is empty in the forward direction;
    /// callers wanting reverse iteration should use [`TimeSeries::new_with_step`]
    /// with a negative [`ExactDuration`].
    EmptyForwardRange,
    /// The end-start duration overflows the i128 nanosecond representation.
    DurationOverflow,
}

impl core::fmt::Display for TimeSeriesError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroStep => f.write_str("TimeSeries step must be non-zero"),
            Self::EmptyForwardRange => {
                f.write_str("TimeSeries::new requires end >= start; use new_with_step with a negative step for descending series")
            }
            Self::DurationOverflow => {
                f.write_str("TimeSeries range exceeds i128 nanosecond capacity")
            }
        }
    }
}

impl std::error::Error for TimeSeriesError {}

impl From<DurationError> for TimeSeriesError {
    fn from(_: DurationError) -> Self {
        Self::DurationOverflow
    }
}

/// Half-open iterator `[start, end)` stepping by an [`ExactDuration`].
///
/// Iteration is **deterministic by index**: the `n`th item is
/// `start.add_exact(step * n)` computed in i128 nanoseconds, NOT by repeated
/// addition. This avoids accumulating split-f64 drift over long ranges.
#[derive(Debug, Clone)]
pub struct TimeSeries<S: CoordinateScale, F: TimeFormat = crate::format::J2000s> {
    start: Time<S, F>,
    #[allow(dead_code)]
    /// Total nanoseconds covered by the half-open range; retained for debug
    /// inspection and future range introspection helpers.
    span_nanos: i128,
    step_nanos: i128,
    /// Items already produced.
    cursor: u64,
    /// Total number of items in this series (precomputed).
    len: u64,
}

impl<S: CoordinateScale, F: TimeFormat> TimeSeries<S, F> {
    /// Build a forward-stepping series `[start, end)` with positive step.
    ///
    /// Returns [`TimeSeriesError::EmptyForwardRange`] if `end < start`,
    /// [`TimeSeriesError::ZeroStep`] if `step.is_zero()`.
    pub fn new(
        start: Time<S, F>,
        end: Time<S, F>,
        step: ExactDuration,
    ) -> Result<Self, TimeSeriesError> {
        if step.is_zero() {
            return Err(TimeSeriesError::ZeroStep);
        }
        let span = end.diff_exact(start)?;
        let span_nanos = span.as_nanos_i128();
        let step_nanos = step.as_nanos_i128();
        if span_nanos == 0 {
            // Empty but valid (zero-length half-open range).
            return Ok(Self {
                start,
                span_nanos: 0,
                step_nanos,
                cursor: 0,
                len: 0,
            });
        }
        // Forward iteration requires the signs of span and step to agree, and
        // the magnitude of |span| / |step| to bound the count.
        if span_nanos.signum() != step_nanos.signum() {
            return Err(TimeSeriesError::EmptyForwardRange);
        }
        // Number of items: ceil(|span| / |step|) for a half-open range with
        // strict containment of every step beyond start, BUT half-open
        // semantics on `end` means we use floor and exclude any final point
        // that would land exactly on `end`. Formally:
        //   n = ceil(span / step)  if step doesn't divide span
        //   n = span / step        otherwise (the last sample equals end, excluded)
        let len = {
            let span_abs = span_nanos.unsigned_abs();
            let step_abs = step_nanos.unsigned_abs();
            let q = span_abs / step_abs;
            let r = span_abs % step_abs;
            if r == 0 {
                q as u64
            } else {
                (q + 1) as u64
            }
        };
        Ok(Self {
            start,
            span_nanos,
            step_nanos,
            cursor: 0,
            len,
        })
    }

    /// Build a series allowing reverse iteration via a negative step.
    /// Range semantics: items satisfy `step > 0 ⇒ start + k·step < end`, or
    /// `step < 0 ⇒ start + k·step > end`.
    pub fn new_with_step(
        start: Time<S, F>,
        end: Time<S, F>,
        step: ExactDuration,
    ) -> Result<Self, TimeSeriesError> {
        Self::new(start, end, step)
    }

    /// Number of items remaining in the series.
    #[inline]
    pub fn remaining(&self) -> u64 {
        self.len.saturating_sub(self.cursor)
    }

    /// Total number of items in the series (independent of cursor).
    #[inline]
    pub fn len_total(&self) -> u64 {
        self.len
    }

    /// True iff this series has produced all items.
    #[inline]
    pub fn is_exhausted(&self) -> bool {
        self.cursor >= self.len
    }

    /// The `n`th item, computed from `start` (NOT by repeated addition).
    /// Returns `None` if `n >= len_total()`.
    pub fn nth_item(&self, n: u64) -> Option<Time<S, F>> {
        if n >= self.len {
            return None;
        }
        let total_nanos = (n as i128).checked_mul(self.step_nanos)?;
        Some(self.start.add_exact(ExactDuration::from_nanos(total_nanos)))
    }
}

impl<S: CoordinateScale, F: TimeFormat> Iterator for TimeSeries<S, F> {
    type Item = Time<S, F>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_exhausted() {
            return None;
        }
        let item = self.nth_item(self.cursor)?;
        self.cursor += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining();
        let cap = remaining.min(usize::MAX as u64) as usize;
        (cap, Some(cap))
    }

    fn count(self) -> usize {
        self.remaining().min(usize::MAX as u64) as usize
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.cursor = self.cursor.saturating_add(n as u64);
        self.next()
    }
}

impl<S: CoordinateScale, F: TimeFormat> ExactSizeIterator for TimeSeries<S, F> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Time, TT};
    use qtty::Second;

    fn t(s: f64) -> Time<TT> {
        Time::<TT>::from_raw_j2000_seconds(Second::new(s)).unwrap()
    }

    #[test]
    fn ten_second_series() {
        let s = TimeSeries::new(t(0.0), t(10.0), ExactDuration::SECOND).unwrap();
        assert_eq!(s.len_total(), 10);
        assert_eq!(s.count(), 10);
    }

    #[test]
    fn zero_step_rejected() {
        assert!(matches!(
            TimeSeries::new(t(0.0), t(10.0), ExactDuration::ZERO),
            Err(TimeSeriesError::ZeroStep)
        ));
    }

    #[test]
    fn empty_forward_range_rejected() {
        assert!(matches!(
            TimeSeries::new(t(10.0), t(0.0), ExactDuration::SECOND),
            Err(TimeSeriesError::EmptyForwardRange)
        ));
    }

    #[test]
    fn empty_zero_span_returns_empty() {
        let s = TimeSeries::new(t(5.0), t(5.0), ExactDuration::SECOND).unwrap();
        assert_eq!(s.len_total(), 0);
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn half_open_excludes_endpoint() {
        let s = TimeSeries::new(t(0.0), t(3.0), ExactDuration::SECOND).unwrap();
        let items: Vec<_> = s.collect();
        assert_eq!(items.len(), 3);
        // Last item should be at t=2 s, not t=3.
        let last = items.last().unwrap();
        let secs = (last.raw_seconds_pair().0 + last.raw_seconds_pair().1).value();
        assert!((secs - 2.0).abs() < 1e-9);
    }

    #[test]
    fn non_dividing_step_yields_ceiling_count() {
        // [0, 3.5) step 1 s → 4 samples at 0, 1, 2, 3
        let s = TimeSeries::new(t(0.0), t(3.5), ExactDuration::SECOND).unwrap();
        assert_eq!(s.len_total(), 4);
    }

    #[test]
    fn nth_item_is_deterministic() {
        let s = TimeSeries::new(t(0.0), t(100.0), ExactDuration::SECOND).unwrap();
        let got = s.nth_item(50).unwrap();
        let secs = (got.raw_seconds_pair().0 + got.raw_seconds_pair().1).value();
        assert!((secs - 50.0).abs() < 1e-9);
        assert!(s.nth_item(100).is_none());
    }

    #[test]
    fn reverse_step_iterates_downward() {
        let s =
            TimeSeries::new_with_step(t(10.0), t(0.0), ExactDuration::from_nanos(-1_000_000_000))
                .unwrap();
        assert_eq!(s.len_total(), 10);
        let items: Vec<_> = s.collect();
        let first = items.first().unwrap();
        let last = items.last().unwrap();
        let first_s = (first.raw_seconds_pair().0 + first.raw_seconds_pair().1).value();
        let last_s = (last.raw_seconds_pair().0 + last.raw_seconds_pair().1).value();
        assert!((first_s - 10.0).abs() < 1e-9);
        assert!((last_s - 1.0).abs() < 1e-9);
    }

    #[test]
    fn skip_via_nth() {
        let mut s = TimeSeries::new(t(0.0), t(10.0), ExactDuration::SECOND).unwrap();
        let third = s.nth(2).unwrap();
        let secs = (third.raw_seconds_pair().0 + third.raw_seconds_pair().1).value();
        assert!((secs - 2.0).abs() < 1e-9);
    }
}
