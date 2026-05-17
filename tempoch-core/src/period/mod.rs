// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Generic periods and interval-list operations.
//!
//! [`Interval`] is a half-open range `[start, end)` over any totally-ordered
//! instant type. It is parameterised on `T: Copy + PartialOrd` so that the
//! same type works for `Time<A>` on any axis as well as for
//! `chrono::DateTime<Utc>`.

use core::fmt;

use crate::Time;

mod error;
pub use error::{InvalidIntervalError, PeriodListError};

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

/// Half-open time interval `[start, end)`.
///
/// Half-open intervals are convenient for period arithmetic because adjacent
/// intervals such as `[a, b)` and `[b, c)` touch without overlapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval<T: Copy + PartialOrd> {
    /// Inclusive lower bound.
    pub start: T,
    /// Exclusive upper bound.
    pub end: T,
}

impl<T> fmt::Display for Interval<T>
where
    T: Copy + PartialOrd + fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {})", self.start, self.end)
    }
}

/// Typed time period on a given scale.
///
/// This is an alias of [`Interval<Time<S>>`](Interval), so all interval
/// operations are available directly on `Period`.
pub type Period<S> = Interval<Time<S>>;

impl<T: Copy + PartialOrd> Interval<T> {
    /// Construct without validation.
    ///
    /// This accepts any `start`/`end` pair, including reversed or NaN-like
    /// endpoints. Prefer [`try_new`](Self::try_new) when the bounds come from
    /// computation or external input.
    #[inline]
    pub fn new<S: Into<T>, E: Into<T>>(start: S, end: E) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }

    /// Validating constructor: rejects `!(start <= end)` (e.g. reversed bounds or unordered floats such as NaN).
    ///
    /// Zero-length intervals where `start == end` are allowed.
    #[inline]
    pub fn try_new<S: Into<T>, E: Into<T>>(start: S, end: E) -> Result<Self, InvalidIntervalError> {
        let start = start.into();
        let end = end.into();
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(InvalidIntervalError::StartAfterEnd)
        }
    }

    // ── Pair operations ──────────────────────────────────────────────────────

    /// Overlap as a half-open range. Returns `None` if the intervals touch
    /// only at a point or do not overlap.
    #[inline]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let start = partial_max(self.start, other.start);
        let end = partial_min(self.end, other.end);
        if start < end {
            Some(Self::new(start, end))
        } else {
            None
        }
    }

    /// Smallest interval covering both `self` and `other`, together with any
    /// gap between them.
    ///
    /// Returns one interval when they overlap or touch, two when they are
    /// strictly disjoint (sorted by start time).
    ///
    /// This pairwise API preserves disjointness instead of forcing a merged
    /// interval that would invent coverage through the gap.
    #[inline]
    pub fn union(&self, other: &Self) -> Vec<Self> {
        if self.start <= other.end && other.start <= self.end {
            vec![Self::new(
                partial_min(self.start, other.start),
                partial_max(self.end, other.end),
            )]
        } else if self.start <= other.start {
            vec![*self, *other]
        } else {
            vec![*other, *self]
        }
    }

    // ── Self-as-outer complement ─────────────────────────────────────────────

    /// Gaps inside `self` that are not covered by any interval in `periods`.
    ///
    /// `periods` must be sorted and non-overlapping
    /// (see [`Interval::validate`]). Intervals outside `self` are effectively
    /// ignored except for how they advance the internal cursor.
    #[inline]
    pub fn complement(&self, periods: &[Self]) -> Vec<Self> {
        let mut gaps = Vec::with_capacity(periods.len().saturating_add(1));
        let mut cursor = self.start;
        for p in periods {
            if p.end <= cursor {
                continue;
            }
            if p.start >= self.end {
                break;
            }
            if p.start > cursor {
                gaps.push(Self::new(cursor, p.start));
            }
            if p.end >= self.end {
                return gaps;
            }
            cursor = p.end;
        }
        if cursor < self.end {
            gaps.push(Self::new(cursor, self.end));
        }
        gaps
    }

    /// Checked variant of [`complement`](Self::complement).
    ///
    /// Validates that `self` is well ordered and that `periods` is sorted,
    /// non-overlapping, and internally valid before computing the complement.
    #[inline]
    pub fn try_complement(&self, periods: &[Self]) -> Result<Vec<Self>, PeriodListError> {
        if self
            .start
            .partial_cmp(&self.end)
            .is_none_or(|ordering| ordering == core::cmp::Ordering::Greater)
        {
            return Err(PeriodListError::InvalidInterval { index: 0 });
        }
        Self::validate(periods)?;
        Ok(self.complement(periods))
    }

    // ── List operations (associated functions) ───────────────────────────────

    /// Check that a list is sorted, non-overlapping, and each `start <= end`.
    ///
    /// Touching intervals such as `[a, b)` followed by `[b, c)` are valid.
    pub fn validate(periods: &[Self]) -> Result<(), PeriodListError> {
        let mut prev: Option<Interval<T>> = None;
        for (i, period) in periods.iter().copied().enumerate() {
            if period
                .start
                .partial_cmp(&period.end)
                .is_none_or(|ordering| ordering == core::cmp::Ordering::Greater)
            {
                return Err(PeriodListError::InvalidInterval { index: i });
            }
            if let Some(previous) = prev {
                if previous
                    .start
                    .partial_cmp(&period.start)
                    .is_none_or(|ordering| ordering == core::cmp::Ordering::Greater)
                {
                    return Err(PeriodListError::Unsorted { index: i });
                }
                if previous.end > period.start {
                    return Err(PeriodListError::Overlapping { index: i });
                }
            }
            prev = Some(period);
        }
        Ok(())
    }

    /// Intersection of two sorted, non-overlapping period lists. `O(n + m)`.
    pub fn intersect_many(a: &[Self], b: &[Self]) -> Vec<Self> {
        let mut result = Vec::with_capacity(a.len().min(b.len()));
        let (mut i, mut j) = (0, 0);
        while i < a.len() && j < b.len() {
            let start = partial_max(a[i].start, b[j].start);
            let end = partial_min(a[i].end, b[j].end);
            if start < end {
                result.push(Self::new(start, end));
            }
            if a[i].end <= b[j].end {
                i += 1;
            } else {
                j += 1;
            }
        }
        result
    }

    /// Checked variant of [`intersect_many`](Self::intersect_many).
    ///
    /// Validates both input lists before computing their intersection.
    pub fn try_intersect_many(a: &[Self], b: &[Self]) -> Result<Vec<Self>, PeriodListError> {
        Self::validate(a)?;
        Self::validate(b)?;
        Ok(Self::intersect_many(a, b))
    }

    /// Union of two sorted period lists.
    ///
    /// Overlapping and adjacent intervals are merged. The result is sorted and
    /// non-overlapping.
    pub fn union_many(a: &[Self], b: &[Self]) -> Vec<Self> {
        let mut combined: Vec<Self> = a.iter().chain(b.iter()).copied().collect();
        combined.sort_unstable_by(|x, y| {
            x.start
                .partial_cmp(&y.start)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        Self::normalize(&combined)
    }

    /// Sort and merge overlapping or adjacent intervals.
    ///
    /// This is useful for normalizing hand-built or concatenated period lists
    /// before later set operations.
    pub fn normalize(periods: &[Self]) -> Vec<Self> {
        if periods.is_empty() {
            return Vec::new();
        }
        let mut sorted: Vec<_> = periods.to_vec();
        sorted.sort_unstable_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        let mut merged = Vec::with_capacity(sorted.len());
        merged.push(sorted[0]);
        for period in sorted.into_iter().skip(1) {
            if let Some(last) = merged.last_mut() {
                if period.start <= last.end {
                    if period.end > last.end {
                        last.end = period.end;
                    }
                } else {
                    merged.push(period);
                }
            }
        }
        merged
    }
}

/// Gaps inside `outer` that are not covered by any interval in `periods`.
///
/// This is a free-function wrapper around [`Interval::complement`].  `periods`
/// must be sorted and non-overlapping (see [`Interval::validate`]).
///
/// # Example
///
/// ```
/// use tempoch_core::{complement_within, JulianDate, TT, Interval};
///
/// let outer = Interval::<JulianDate<TT>>::new(
///     JulianDate::new(2_451_545.0),
///     JulianDate::new(2_451_550.0),
/// );
/// let filled = vec![Interval::new(
///     JulianDate::new(2_451_546.0),
///     JulianDate::new(2_451_548.0),
/// )];
/// let gaps = complement_within(outer, &filled);
/// assert_eq!(gaps.len(), 2);
/// ```
#[inline]
pub fn complement_within<T: Copy + PartialOrd>(
    outer: Interval<T>,
    periods: &[Interval<T>],
) -> Vec<Interval<T>> {
    outer.complement(periods)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "serde")]
    use crate::format::JulianDate;
    use crate::format::{ModifiedJulianDate, MJD};
    #[cfg(feature = "serde")]
    use crate::Time;
    use crate::TT;
    use qtty::Day;
    #[cfg(feature = "serde")]
    use serde::de::IntoDeserializer;
    #[cfg(feature = "serde")]
    use serde::Deserialize;
    #[cfg(feature = "serde")]
    use serde_json::json;

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
        let gaps = outer.complement(&inside);
        assert_eq!(gaps.len(), 3);
        assert_eq!(gaps[0], Interval::<f64>::new(0.0, 1.0));
        assert_eq!(gaps[1], Interval::<f64>::new(2.0, 4.0));
        assert_eq!(gaps[2], Interval::<f64>::new(6.0, 10.0));
    }

    #[test]
    fn complement_ignores_periods_after_outer_interval() {
        let outer = Interval::<f64>::new(10.0_f64, 20.0);
        let inside = vec![Interval::<f64>::new(25.0_f64, 30.0)];

        assert_eq!(
            outer.complement(&inside),
            vec![Interval::<f64>::new(10.0, 20.0)]
        );
    }

    #[test]
    fn complement_clips_periods_spanning_outer_end() {
        let outer = Interval::<f64>::new(10.0_f64, 20.0);
        let inside = vec![Interval::<f64>::new(12.0_f64, 30.0)];

        assert_eq!(
            outer.complement(&inside),
            vec![Interval::<f64>::new(10.0, 12.0)]
        );
    }

    #[test]
    fn intersect_merge() {
        let a = vec![
            Interval::<f64>::new(0.0_f64, 5.0),
            Interval::<f64>::new(10.0, 15.0),
        ];
        let b = vec![Interval::<f64>::new(3.0_f64, 12.0)];
        let ix = Interval::intersect_many(&a, &b);
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
        let merged = Interval::normalize(&input);
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
            Interval::validate(&periods),
            Err(PeriodListError::Overlapping { index: 1 })
        );
    }

    #[test]
    fn period_accepts_typed_times() {
        let p = Period::<TT>::new(
            ModifiedJulianDate::<TT>::new(51_544.5).to_j2000s(),
            ModifiedJulianDate::<TT>::new(51_545.25).to_j2000s(),
        );
        assert_eq!(p.start.to::<MJD>().raw(), Day::new(51_544.5));
        assert_eq!(p.end.to::<MJD>().raw(), Day::new(51_545.25));
    }

    #[test]
    fn display_invalid_interval_error() {
        let e = InvalidIntervalError::StartAfterEnd;
        assert!(e.to_string().contains("start"));
    }

    #[test]
    fn display_period_list_errors() {
        assert!(PeriodListError::InvalidInterval { index: 2 }
            .to_string()
            .contains("2"));
        assert!(PeriodListError::Unsorted { index: 3 }
            .to_string()
            .contains("3"));
        assert!(PeriodListError::Overlapping { index: 4 }
            .to_string()
            .contains("4"));
    }

    #[test]
    fn normalize_empty_returns_empty() {
        let result = Interval::<f64>::normalize(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn validate_detects_invalid_interval() {
        // Interval::new does not validate; create one with start > end directly
        let periods = vec![Interval::<f64> {
            start: 5.0,
            end: 1.0,
        }];
        assert_eq!(
            Interval::validate(&periods),
            Err(PeriodListError::InvalidInterval { index: 0 })
        );
    }

    #[test]
    fn validate_detects_unsorted() {
        let periods = vec![
            Interval::<f64>::new(5.0_f64, 8.0),
            Interval::<f64>::new(1.0_f64, 4.0),
        ];
        assert_eq!(
            Interval::validate(&periods),
            Err(PeriodListError::Unsorted { index: 1 })
        );
    }

    #[test]
    fn checked_complement_rejects_invalid_inputs() {
        let outer = Interval::<f64>::new(0.0_f64, 10.0);
        let periods = vec![
            Interval::<f64>::new(5.0_f64, 8.0),
            Interval::<f64>::new(1.0_f64, 4.0),
        ];
        assert_eq!(
            outer.try_complement(&periods),
            Err(PeriodListError::Unsorted { index: 1 })
        );

        let invalid_outer = Interval::<f64>::new(10.0_f64, 0.0);
        assert_eq!(
            invalid_outer.try_complement(&[]),
            Err(PeriodListError::InvalidInterval { index: 0 })
        );
    }

    #[test]
    fn checked_intersection_matches_unchecked_for_valid_inputs() {
        let a = vec![Interval::<f64>::new(0.0_f64, 5.0)];
        let b = vec![Interval::<f64>::new(3.0_f64, 7.0)];
        assert_eq!(
            Interval::try_intersect_many(&a, &b).unwrap(),
            Interval::intersect_many(&a, &b)
        );
    }

    #[test]
    fn union_pair_overlapping() {
        let a = Interval::<f64>::new(0.0_f64, 5.0);
        let b = Interval::<f64>::new(3.0_f64, 8.0);
        let u = a.union(&b);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0], Interval::<f64>::new(0.0, 8.0));
    }

    #[test]
    fn union_pair_disjoint() {
        let a = Interval::<f64>::new(0.0_f64, 3.0);
        let b = Interval::<f64>::new(5.0_f64, 8.0);
        let u = a.union(&b);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0], Interval::<f64>::new(0.0, 3.0));
        assert_eq!(u[1], Interval::<f64>::new(5.0, 8.0));
    }

    #[test]
    fn union_many_merges_two_lists() {
        let a = vec![
            Interval::<f64>::new(0.0_f64, 3.0),
            Interval::<f64>::new(7.0, 9.0),
        ];
        let b = vec![Interval::<f64>::new(2.0_f64, 5.0)];
        let u = Interval::union_many(&a, &b);
        assert_eq!(u.len(), 2);
        assert_eq!(u[0], Interval::<f64>::new(0.0, 5.0));
        assert_eq!(u[1], Interval::<f64>::new(7.0, 9.0));
    }

    #[test]
    fn display_formats_periods_via_endpoint_display() {
        let mjd = Period::<TT>::new(
            ModifiedJulianDate::<TT>::new(51_544.5).to_j2000s(),
            ModifiedJulianDate::<TT>::new(51_545.25).to_j2000s(),
        );

        assert!(mjd.to_string().contains("TT"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrips_period_shapes() {
        let mjd = Period::<TT>::new(
            ModifiedJulianDate::<TT>::new(51_544.5).to_j2000s(),
            ModifiedJulianDate::<TT>::new(51_545.25).to_j2000s(),
        );
        let jd = Period::<TT>::new(
            JulianDate::<TT>::new(2_451_545.0).to_j2000s(),
            JulianDate::<TT>::new(2_451_546.0).to_j2000s(),
        );
        let native = Period::<TT>::new(
            Time::<TT>::from_raw_j2000_seconds(qtty::Second::new(100.0)).unwrap(),
            Time::<TT>::from_raw_j2000_seconds(qtty::Second::new(200.0)).unwrap(),
        );

        let mjd_json = serde_json::to_value(mjd).unwrap();
        assert_eq!(serde_json::from_value::<Period<TT>>(mjd_json).unwrap(), mjd);

        let jd_json = serde_json::to_value(jd).unwrap();
        assert_eq!(serde_json::from_value::<Period<TT>>(jd_json).unwrap(), jd);

        let native_json = serde_json::to_value(native).unwrap();
        assert_eq!(
            serde_json::from_value::<Period<TT>>(native_json).unwrap(),
            native
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_rejects_reversed_periods() {
        let err = serde_json::from_value::<Period<TT>>(json!({
            "start": 10.0,
            "end": 9.0,
        }))
        .unwrap_err();
        assert!(err.to_string().contains("start"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_rejects_nan_scalar_time_values() {
        let res: Result<Time<TT>, serde::de::value::Error> =
            Time::<TT>::deserialize(f64::NAN.into_deserializer());
        let err = res.expect_err("nan scalar");
        assert!(
            err.to_string().contains("NaN"),
            "unexpected serde error message: {err}"
        );

        let err = serde_json::from_value::<Period<TT>>(json!({
            "start": serde_json::Value::Null,
            "end": 5.0,
        }))
        .expect_err("null start");
        assert!(
            err.to_string().contains("null"),
            "unexpected serde error message: {err}"
        );
    }
}
