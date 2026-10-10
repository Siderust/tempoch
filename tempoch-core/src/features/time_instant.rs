// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Feature-style extension traits for time-adjacent algorithms.

use crate::qtty::Day;

use crate::format::{JulianDate, ModifiedJulianDate};
use crate::model::scale::{CoordinateScale, TT};
use crate::model::time::Time;
use crate::qtty::Second;

/// Provides arithmetic on [`Time<S>`] values via seconds duration.
///
/// Used by root-finding algorithms that bisect over a time interval. For
/// encoded JD/MJD values, arithmetic follows the underlying physical instant:
/// adding one `Day` adds 86,400 SI seconds, not one civil calendar day. Across
/// a UTC leap second those are different operations.
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
        *self - *other
    }

    #[inline]
    fn add_duration(&self, duration: Day) -> Self {
        *self + duration
    }
}

impl<S: CoordinateScale> TimeInstant for JulianDate<S> {
    type Duration = Day;

    #[inline]
    fn difference(&self, other: &Self) -> Day {
        *self - *other
    }

    #[inline]
    fn add_duration(&self, duration: Day) -> Self {
        *self + duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{J2000Seconds, JulianDate, ModifiedJulianDate};
    use crate::model::scale::{TT, UTC};

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

        let ctx = crate::TimeContext::new();
        let utc = ModifiedJulianDate::<UTC>::try_new_with(Day::new(57_753.0), &ctx).unwrap();
        let utc_later = utc.add_duration(Day::new(1.0));
        assert_eq!(utc_later.difference(&utc), Day::new(1.0));
        let elapsed = utc_later.to_j2000s();
        assert!(elapsed.is_leap_second_with(&ctx));

        let next_midnight =
            ModifiedJulianDate::<UTC>::try_new_with(Day::new(57_754.0), &ctx).unwrap();
        let civil_day_elapsed = next_midnight.to_j2000s() - utc.to_j2000s();
        assert_eq!(civil_day_elapsed, Second::new(86_401.0));
        assert_eq!(
            next_midnight.difference(&utc),
            Day::new(86_401.0 / 86_400.0)
        );
    }
}
