// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Generic time intervals.
//!
//! [`Interval`] is a half-open range `[start, end)` over any totally-ordered
//! instant type. It is parameterised on `T: Copy + PartialOrd` so that the
//! same type works for `Time<A>` on any axis as well as for
//! `chrono::DateTime<Utc>`.

use core::fmt;

use crate::{J2000s, Time};

#[inline]
fn partial_max<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a >= b {
        a
    } else {
        b
    }
}

#[inline]
fn partial_min<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a <= b {
        a
    } else {
        b
    }
}

/// Error constructing an [`Interval`] with invalid bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidIntervalError {
    /// `!(start <= end)` (also triggers for NaN endpoints).
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

/// Half-open time interval `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval<T: Copy + PartialOrd> {
    pub start: T,
    pub end: T,
}

/// Typed time period on a given scale and format.
///
/// This is an alias of [`Interval<Time<S, F>>`](Interval), so all interval
/// operations are available directly on `Period`.
pub type Period<S, F = J2000s> = Interval<Time<S, F>>;

/// Backward-compatible alias for period construction validation errors.
pub type InvalidPeriodError = InvalidIntervalError;

impl<T: Copy + PartialOrd> Interval<T> {
    /// Construct without validation. Prefer [`try_new`](Self::try_new) for
    /// computed inputs.
    #[inline]
    pub fn new<S: Into<T>, E: Into<T>>(start: S, end: E) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }

    /// Validating constructor: rejects `start > end` and NaN endpoints.
    #[inline]
    pub fn try_new<S: Into<T>, E: Into<T>>(
        start: S,
        end: E,
    ) -> Result<Self, InvalidIntervalError> {
        let start = start.into();
        let end = end.into();
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(InvalidIntervalError::StartAfterEnd)
        }
    }

    /// Overlap as a half-open range. Returns `None` if the intervals touch
    /// only at a point or do not overlap.
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let start = partial_max(self.start, other.start);
        let end = partial_min(self.end, other.end);
        if start < end {
            Some(Self::new(start, end))
        } else {
            None
        }
    }
}

/// Gaps (complement) of `periods` within `outer`. `periods` must be sorted
/// and non-overlapping (see [`validate_period_list`]).
pub fn complement_within<T: Copy + PartialOrd>(
    outer: Interval<T>,
    periods: &[Interval<T>],
) -> Vec<Interval<T>> {
    let mut gaps = Vec::new();
    let mut cursor = outer.start;
    for p in periods {
        if p.start > cursor {
            gaps.push(Interval::new(cursor, p.start));
        }
        if p.end > cursor {
            cursor = p.end;
        }
    }
    if cursor < outer.end {
        gaps.push(Interval::new(cursor, outer.end));
    }
    gaps
}

/// Intersection of two sorted, non-overlapping period lists. `O(n + m)`.
pub fn intersect_periods<T: Copy + PartialOrd>(
    a: &[Interval<T>],
    b: &[Interval<T>],
) -> Vec<Interval<T>> {
    let mut result = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        let start = partial_max(a[i].start, b[j].start);
        let end = partial_min(a[i].end, b[j].end);
        if start < end {
            result.push(Interval::new(start, end));
        }
        if a[i].end <= b[j].end {
            i += 1;
        } else {
            j += 1;
        }
    }
    result
}

/// Check that a list is sorted, non-overlapping, and each `start <= end`.
pub fn validate_period_list<T: Copy + PartialOrd>(
    periods: &[Interval<T>],
) -> Result<(), PeriodListError> {
    for (i, p) in periods.iter().enumerate() {
        if p.start
            .partial_cmp(&p.end)
            .is_none_or(|o| o == core::cmp::Ordering::Greater)
        {
            return Err(PeriodListError::InvalidInterval { index: i });
        }
    }
    for i in 1..periods.len() {
        if periods[i - 1]
            .start
            .partial_cmp(&periods[i].start)
            .is_none_or(|o| o == core::cmp::Ordering::Greater)
        {
            return Err(PeriodListError::Unsorted { index: i });
        }
        if periods[i - 1].end > periods[i].start {
            return Err(PeriodListError::Overlapping { index: i });
        }
    }
    Ok(())
}

/// Sort and merge overlapping/adjacent intervals.
pub fn normalize_periods<T: Copy + PartialOrd>(periods: &[Interval<T>]) -> Vec<Interval<T>> {
    if periods.is_empty() {
        return Vec::new();
    }
    let mut sorted: Vec<_> = periods.to_vec();
    sorted.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    let mut merged = vec![sorted[0]];
    for p in &sorted[1..] {
        let last = merged.last_mut().unwrap();
        if p.start <= last.end {
            if p.end > last.end {
                last.end = p.end;
            }
        } else {
            merged.push(*p);
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mjd, TT};
    use qtty::Day;

    #[test]
    fn try_new_rejects_reversed() {
        assert_eq!(
            Interval::<f64>::try_new(2.0_f64, 1.0).unwrap_err(),
            InvalidIntervalError::StartAfterEnd
        );
    }

    #[test]
    fn try_new_rejects_nan() {
        assert!(Interval::<f64>::try_new(f64::NAN, 0.0).is_err());
    }

    #[test]
    fn intersection_half_open() {
        let a = Interval::<f64>::new(0.0_f64, 10.0);
        let b = Interval::<f64>::new(10.0, 20.0);
        assert!(a.intersection(&b).is_none());
        let c = Interval::<f64>::new(5.0_f64, 15.0);
        let x = a.intersection(&c).unwrap();
        assert_eq!(x.start, 5.0);
        assert_eq!(x.end, 10.0);
    }

    #[test]
    fn complement_covers_gaps() {
        let outer = Interval::<f64>::new(0.0_f64, 10.0);
        let inside = vec![
            Interval::<f64>::new(1.0_f64, 2.0),
            Interval::<f64>::new(4.0, 6.0),
        ];
        let gaps = complement_within(outer, &inside);
        assert_eq!(gaps.len(), 3);
        assert_eq!(gaps[0], Interval::<f64>::new(0.0, 1.0));
        assert_eq!(gaps[1], Interval::<f64>::new(2.0, 4.0));
        assert_eq!(gaps[2], Interval::<f64>::new(6.0, 10.0));
    }

    #[test]
    fn intersect_merge() {
        let a = vec![
            Interval::<f64>::new(0.0_f64, 5.0),
            Interval::<f64>::new(10.0, 15.0),
        ];
        let b = vec![Interval::<f64>::new(3.0_f64, 12.0)];
        let ix = intersect_periods(&a, &b);
        assert_eq!(ix.len(), 2);
        assert_eq!(ix[0], Interval::<f64>::new(3.0, 5.0));
        assert_eq!(ix[1], Interval::<f64>::new(10.0, 12.0));
    }

    #[test]
    fn normalize_merges_overlap() {
        let input = vec![
            Interval::<f64>::new(5.0_f64, 8.0),
            Interval::<f64>::new(0.0, 3.0),
            Interval::<f64>::new(2.0, 6.0),
        ];
        let merged = normalize_periods(&input);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], Interval::<f64>::new(0.0, 8.0));
    }

    #[test]
    fn validate_detects_overlap() {
        let periods = vec![
            Interval::<f64>::new(0.0_f64, 5.0),
            Interval::<f64>::new(3.0, 8.0),
        ];
        assert_eq!(
            validate_period_list(&periods),
            Err(PeriodListError::Overlapping { index: 1 })
        );
    }

    #[test]
    fn period_accepts_raw_mjd_values() {
        let p = Period::<TT, Mjd>::new(51_544.5, 51_545.25);
        assert_eq!(p.start.modified_julian_days(), Day::new(51_544.5));
        assert_eq!(p.end.modified_julian_days(), Day::new(51_545.25));
    }
}
