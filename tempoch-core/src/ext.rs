// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Extension traits for time types.

use qtty::Day;

use crate::representation::{JulianDate, ModifiedJulianDate};
use crate::scale::CoordinateScale;
use crate::time::Time;
use crate::scale::TT;
use qtty::Second;

/// Provides arithmetic on [`Time<S>`] values via seconds duration.
///
/// Used by root-finding algorithms that bisect over a time interval.
pub trait TimeInstant: Copy + PartialOrd {
    /// Duration type produced by subtracting two instants.
    type Duration;

    /// Signed duration from `other` to `self` (`self − other`).
    fn difference(&self, other: &Self) -> Self::Duration;

    /// Shift this instant forward by `duration`.
    fn add_duration(&self, duration: Self::Duration) -> Self;
}

impl TimeInstant for Time<TT> {
    type Duration = Second;

    #[inline]
    fn difference(&self, other: &Self) -> Second {
        *self - *other
    }

    #[inline]
    fn add_duration(&self, dur: Second) -> Self {
        *self + dur
    }
}

impl<S: CoordinateScale> TimeInstant for ModifiedJulianDate<S> {
    type Duration = Day;

    #[inline]
    fn difference(&self, other: &Self) -> Day {
        Day::new(self.raw().value() - other.raw().value())
    }

    #[inline]
    fn add_duration(&self, duration: Day) -> Self {
        ModifiedJulianDate::<S>::new_unchecked(Day::new(self.raw().value() + duration.value()))
    }
}

impl<S: CoordinateScale> TimeInstant for JulianDate<S> {
    type Duration = Day;

    #[inline]
    fn difference(&self, other: &Self) -> Day {
        Day::new(self.raw().value() - other.raw().value())
    }

    #[inline]
    fn add_duration(&self, duration: Day) -> Self {
        JulianDate::<S>::new_unchecked(Day::new(self.raw().value() + duration.value()))
    }
}
