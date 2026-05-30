// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Encoding helpers: centralized epoch and coordinate arithmetic.
//!
//! These functions own the J2000-offset, JD/MJD, and civil-epoch
//! arithmetic so the rest of the crate expresses intent rather than
//! manual coordinate algebra.
//!
//! Epoch constants remain the single source of truth in
//! [`constats`](crate::foundation::constats); call sites reference helpers here
//! instead of duplicating offset formulas.
//!
//! Format marker types (`JD`, `MJD`, `J2000s`, `Unix`, `GPS`) live in
//! [`crate::format::markers`]; this module only owns the arithmetic.
//!
//! # Taxonomy
//!
//! The helpers here fall into three categories:
//!
//! * **Coordinate encodings** — JD, MJD, SI seconds, Julian centuries.
//!   Pure arithmetic on the J2000 TT epoch.
//! * **Civil/transport encodings** — POSIX (Unix) seconds, GPS seconds.
//!   Involve civil epoch offsets; the civil layer adds UTC-TAI history
//!   on top.
//! * **Convenience** — `jd_to_mjd` for axis-independent
//!   day-count conversions.

use crate::format::{TimeFormat, JD, MJD};
use crate::foundation::constats::{J2000_JD_TT_DAY, JD_MINUS_MJD};
use affn::algebra::{AffineMap1, Point1, Space};
use qtty::unit::{Day as DayUnit, Second as SecondUnit};
use qtty::{Day, Second};

/// Sealed trait for day-based time formats with a well-defined J2000 TT
/// origin expressed in their own day coordinate.  Implementing this trait
/// is sufficient to obtain the generic `day_to_j2000_seconds` /
/// `j2000_seconds_to_day` converters for free.
pub(crate) trait DayEncoding: TimeFormat<Unit = DayUnit> {
    fn j2000_origin() -> Day;
}

impl DayEncoding for JD {
    fn j2000_origin() -> Day {
        J2000_JD_TT_DAY
    }
}

impl DayEncoding for MJD {
    fn j2000_origin() -> Day {
        J2000_JD_TT_DAY - JD_MINUS_MJD
    }
}

#[derive(Debug, Copy, Clone)]
struct SourceDayAxis;
impl Space for SourceDayAxis {}

#[derive(Debug, Copy, Clone)]
struct TargetDayAxis;
impl Space for TargetDayAxis {}

#[inline]
pub(crate) fn affine_day_coordinate(source: Day, source_origin: Day, target_origin: Day) -> Day {
    let map =
        AffineMap1::<SourceDayAxis, TargetDayAxis, DayUnit>::new(source_origin, target_origin, 1.0);
    map.apply_point(Point1::<SourceDayAxis, DayUnit>::new(source))
        .x()
}

/// Day-based time format value → SI seconds since J2000 TT.
#[inline]
pub(crate) fn day_to_j2000_seconds<F: DayEncoding>(day: Day) -> Second {
    affine_day_coordinate(day, F::j2000_origin(), Day::new(0.0)).to::<SecondUnit>()
}

/// SI seconds since J2000 TT → day-based time format value.
#[inline]
pub(crate) fn j2000_seconds_to_day<F: DayEncoding>(seconds: Second) -> Day {
    affine_day_coordinate(seconds.to::<DayUnit>(), Day::new(0.0), F::j2000_origin())
}

mod jd;
mod mjd;

pub(crate) use jd::jd_to_julian_centuries;
pub(crate) use mjd::{jd_to_mjd, mjd_to_unix_seconds, unix_seconds_to_jd, unix_seconds_to_mjd};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::constats::J2000_JD_TT_DAY;
    use qtty::Second;

    const EPS_S: Second = Second::new(1e-9);
    const EPS_D: Day = Day::new(1e-15);

    #[test]
    fn jd_j2000_round_trip() {
        let jd = Day::new(2_451_545.5);
        let secs = day_to_j2000_seconds::<JD>(jd);
        let back = j2000_seconds_to_day::<JD>(secs);
        assert!((back - jd).abs() < EPS_D);
    }

    #[test]
    fn mjd_j2000_round_trip() {
        let mjd = Day::new(51_544.5);
        let secs = day_to_j2000_seconds::<MJD>(mjd);
        let back = j2000_seconds_to_day::<MJD>(secs);
        assert!((back - mjd).abs() < EPS_D);
    }

    #[test]
    fn j2000_epoch_is_zero_seconds() {
        let secs = day_to_j2000_seconds::<JD>(J2000_JD_TT_DAY);
        assert!(secs.abs() < EPS_S);
    }

    #[test]
    fn julian_centuries_one_century() {
        let jd = J2000_JD_TT_DAY + qtty::time::JULIAN_CENTURY.to::<qtty::unit::Day>();
        let t = jd_to_julian_centuries(jd);
        assert!((t - 1.0).abs() < 1e-12);
    }

    #[test]
    fn unix_mjd_round_trip() {
        let mjd = Day::new(40_587.0); // Unix epoch
        let secs = mjd_to_unix_seconds(mjd);
        assert!(secs.abs() < EPS_S);
        let back = unix_seconds_to_mjd(secs);
        assert!((back - mjd).abs() < EPS_D);
    }
}
