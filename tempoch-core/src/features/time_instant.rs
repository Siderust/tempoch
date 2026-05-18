// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Feature-style extension traits for time-adjacent algorithms.

use qtty::Day;

use crate::format::{JulianDate, ModifiedJulianDate};
use crate::format::{JD, MJD};
use crate::model::scale::CoordinateScale;
use crate::model::scale::TT;
use crate::model::time::Time;
use crate::InfallibleFormatForScale;
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
        MJD::into_time(Day::new(self.raw().value() + duration.value()))
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
        JD::into_time(Day::new(self.raw().value() + duration.value()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{J2000Seconds, JulianDate, ModifiedJulianDate};
    use crate::model::scale::TT;

    #[test]
    fn time_instant_trait_supports_time_and_encoded_dates() {
        let tt = J2000Seconds::<TT>::new(10.0).to_j2000s();
        let tt_later = tt.add_duration(Second::new(2.5));
        assert_eq!(tt_later.difference(&tt), Second::new(2.5));

        let mjd = ModifiedJulianDate::<TT>::new(60_000.0);
        let mjd_later = mjd.add_duration(Day::new(1.25));
        assert_eq!(mjd_later.difference(&mjd), Day::new(1.25));

        let jd = JulianDate::<TT>::new(2_460_000.0);
        let jd_later = jd.add_duration(Day::new(0.5));
        assert_eq!(jd_later.difference(&jd), Day::new(0.5));
    }
}
