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
#[cfg(feature = "serde")]
use serde::ser::SerializeStruct;
#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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

#[cfg(feature = "serde")]
impl<T> Serialize for Interval<T>
where
    T: Copy + PartialOrd + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Interval", 2)?;
        state.serialize_field("start", &self.start)?;
        state.serialize_field("end", &self.end)?;
        state.end()
    }
}

#[cfg(feature = "serde")]
impl<'de, T> Deserialize<'de> for Interval<T>
where
    T: Copy + PartialOrd + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawInterval<T> {
            start: T,
            end: T,
        }

        let raw = RawInterval::<T>::deserialize(deserializer)?;
        Self::try_new(raw.start, raw.end).map_err(serde::de::Error::custom)
    }
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
    pub fn try_new<S: Into<T>, E: Into<T>>(start: S, end: E) -> Result<Self, InvalidIntervalError> {
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
}

/// Gaps (complement) of `periods` within `outer`. `periods` must be sorted
/// and non-overlapping (see [`validate_period_list`]).
pub fn complement_within<T: Copy + PartialOrd>(
    outer: Interval<T>,
    periods: &[Interval<T>],
) -> Vec<Interval<T>> {
    let mut gaps = Vec::with_capacity(periods.len().saturating_add(1));
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
    let mut result = Vec::with_capacity(a.len().min(b.len()));
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

/// Sort and merge overlapping/adjacent intervals.
pub fn normalize_periods<T: Copy + PartialOrd>(periods: &[Interval<T>]) -> Vec<Interval<T>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Jd, Mjd, TT};
    use qtty::Day;
    #[cfg(feature = "serde")]
    use serde::de::{value, IntoDeserializer};
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
        let result = normalize_periods::<f64>(&[]);
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
            validate_period_list(&periods),
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
            validate_period_list(&periods),
            Err(PeriodListError::Unsorted { index: 1 })
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_roundtrips_period_shapes() {
        let mjd = Period::<TT, Mjd>::new(51_544.5, 51_545.25);
        let jd = Period::<TT, Jd>::new(2_451_545.0, 2_451_546.0);
        let native = Period::<TT>::new(100.0, 200.0);

        assert_eq!(
            serde_json::to_value(mjd).unwrap(),
            json!({"start": 51_544.5, "end": 51_545.25})
        );
        assert_eq!(
            serde_json::to_value(jd).unwrap(),
            json!({"start": 2_451_545.0, "end": 2_451_546.0})
        );
        assert_eq!(
            serde_json::to_value(native).unwrap(),
            json!({"start": 100.0, "end": 200.0})
        );

        assert_eq!(
            serde_json::from_value::<Period<TT, Mjd>>(json!({"start": 51_544.5, "end": 51_545.25}))
                .unwrap(),
            mjd
        );
        assert_eq!(
            serde_json::from_value::<Period<TT, Jd>>(json!({"start": 2_451_545.0, "end": 2_451_546.0}))
                .unwrap(),
            jd
        );
        assert_eq!(
            serde_json::from_value::<Period<TT>>(json!({"start": 100.0, "end": 200.0})).unwrap(),
            native
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_rejects_reversed_periods() {
        let err = serde_json::from_value::<Period<TT, Mjd>>(json!({"start": 10.0, "end": 9.0}))
            .unwrap_err();
        assert!(err.to_string().contains("start"));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_rejects_nonfinite_nested_time_values() {
        let entries = vec![
            (
                "start".into_deserializer(),
                f64::NAN.into_deserializer(),
            ),
            (
                "end".into_deserializer(),
                5.0_f64.into_deserializer(),
            ),
        ];
        let deserializer = value::MapDeserializer::<_, value::Error>::new(entries.into_iter());
        let err = Period::<TT, Mjd>::deserialize(deserializer).unwrap_err();
        assert!(err.to_string().contains("finite"));
    }
}
