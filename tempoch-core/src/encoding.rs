// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Encoding helpers: centralized epoch and coordinate arithmetic.
//!
//! These functions own the J2000-offset, JD/MJD, and civil-epoch
//! arithmetic so the rest of the crate expresses intent rather than
//! manual coordinate algebra.
//!
//! Epoch constants remain the single source of truth in
//! [`constats`](super::constats); call sites reference helpers here
//! instead of duplicating offset formulas.
//!
//! # Taxonomy
//!
//! The helpers here fall into three categories that mirror the split
//! proposed in the design-smells note:
//!
//! * **Coordinate encodings** — JD, MJD, SI seconds, Julian centuries.
//!   Pure arithmetic on the J2000 TT epoch.
//! * **Civil/transport encodings** — POSIX (Unix) seconds, GPS seconds.
//!   Involve civil epoch offsets; the civil layer adds UTC-TAI history
//!   on top.
//! * **Convenience** — `jd_to_mjd` for axis-independent
//!   day-count conversions.

use crate::constats::{DAYS_PER_JC, J2000_JD_TT, JD_MINUS_MJD, UNIX_EPOCH_JD, UNIX_EPOCH_MJD};
use affn::algebra::{AffineMap1, Point1, Space};
use qtty::unit::{Day as DayUnit, Second as SecondUnit};
use qtty::{Day, Second};

#[derive(Debug, Copy, Clone)]
struct SourceDayAxis;
impl Space for SourceDayAxis {}

#[derive(Debug, Copy, Clone)]
struct TargetDayAxis;
impl Space for TargetDayAxis {}

#[inline]
fn affine_day_coordinate(source: Day, source_origin: Day, target_origin: Day) -> Day {
    let map = AffineMap1::<SourceDayAxis, TargetDayAxis, DayUnit>::new(
        source_origin,
        target_origin,
        1.0,
    );
    map.apply_point(Point1::<SourceDayAxis, DayUnit>::new(source))
        .x()
}

// ── JD ↔ J2000 seconds ──────────────────────────────────────────────────

/// Julian Day → SI seconds since J2000 TT.
#[inline]
pub(crate) fn jd_to_j2000_seconds(jd: Day) -> Second {
    affine_day_coordinate(jd, J2000_JD_TT, Day::new(0.0)).to::<SecondUnit>()
}

/// SI seconds since J2000 TT → Julian Day.
#[inline]
pub(crate) fn j2000_seconds_to_jd(seconds: Second) -> Day {
    affine_day_coordinate(seconds.to::<DayUnit>(), Day::new(0.0), J2000_JD_TT)
}

// ── MJD ↔ J2000 seconds ─────────────────────────────────────────────────

/// Modified Julian Day → SI seconds since J2000 TT.
#[inline]
pub(crate) fn mjd_to_j2000_seconds(mjd: Day) -> Second {
    affine_day_coordinate(mjd, J2000_JD_TT - JD_MINUS_MJD, Day::new(0.0)).to::<SecondUnit>()
}

/// SI seconds since J2000 TT → Modified Julian Day.
#[inline]
pub(crate) fn j2000_seconds_to_mjd(seconds: Second) -> Day {
    affine_day_coordinate(
        seconds.to::<DayUnit>(),
        Day::new(0.0),
        J2000_JD_TT - JD_MINUS_MJD,
    )
}

// ── JD ↔ MJD ─────────────────────────────────────────────────────────────

/// Julian Day → Modified Julian Day.
#[inline]
pub(crate) fn jd_to_mjd(jd: Day) -> Day {
    jd - JD_MINUS_MJD
}

// ── Julian centuries ─────────────────────────────────────────────────────

/// Julian Day TT → Julian centuries since J2000 TT (dimensionless).
#[inline]
pub(crate) fn jd_to_julian_centuries(jd: Day) -> f64 {
    (jd - J2000_JD_TT) / DAYS_PER_JC
}

// ── Unix / POSIX ─────────────────────────────────────────────────────────

/// UTC MJD → seconds since Unix epoch (1970-01-01).
#[inline]
pub(crate) fn mjd_to_unix_seconds(mjd: Day) -> Second {
    (mjd - UNIX_EPOCH_MJD).to::<SecondUnit>()
}

/// Seconds since Unix epoch → UTC MJD.
#[inline]
pub(crate) fn unix_seconds_to_mjd(seconds: Second) -> Day {
    UNIX_EPOCH_MJD + seconds.to::<DayUnit>()
}

/// Seconds since Unix epoch → Julian Day (UTC axis).
#[inline]
pub(crate) fn unix_seconds_to_jd(seconds: Second) -> Day {
    UNIX_EPOCH_JD + seconds.to::<DayUnit>()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS_S: Second = Second::new(1e-9);
    const EPS_D: Day = Day::new(1e-15);

    #[test]
    fn jd_j2000_round_trip() {
        let jd = Day::new(2_451_545.5);
        let secs = jd_to_j2000_seconds(jd);
        let back = j2000_seconds_to_jd(secs);
        assert!((back - jd).abs() < EPS_D);
    }

    #[test]
    fn mjd_j2000_round_trip() {
        let mjd = Day::new(51_544.5);
        let secs = mjd_to_j2000_seconds(mjd);
        let back = j2000_seconds_to_mjd(secs);
        assert!((back - mjd).abs() < EPS_D);
    }

    #[test]
    fn j2000_epoch_is_zero_seconds() {
        let secs = jd_to_j2000_seconds(J2000_JD_TT);
        assert!(secs.abs() < EPS_S);
    }

    #[test]
    fn julian_centuries_one_century() {
        let jd = J2000_JD_TT + DAYS_PER_JC;
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
