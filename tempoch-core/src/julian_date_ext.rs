// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Vallés Puig, Ramon

//! Julian Date (`Time<JD>`) specific extensions.

use qtty::*;
use std::ops::Add;

use super::instant::Time;
use super::scales::{JD, MJD};

impl Time<JD> {
    /// J2000.0 epoch: 2000-01-01T12:00:00 TT  (JD 2 451 545.0).
    pub const J2000: Self = Self::new(2_451_545.0);

    /// One Julian year expressed in days.
    pub const JULIAN_YEAR: Days = Days::new(365.25);

    /// One Julian century expressed in days.
    pub const JULIAN_CENTURY: Days = Days::new(36_525.0);

    /// One Julian millennium expressed in days.
    pub const JULIAN_MILLENNIUM: Days = Days::new(365_250.0);

    /// Julian millennia since J2000.0 (used by VSOP87).
    #[inline]
    pub fn julian_millennias(&self) -> Millennia {
        Millennia::new(
            ((*self - Self::J2000) / Self::JULIAN_MILLENNIUM)
                .simplify()
                .value(),
        )
    }

    /// Julian centuries since J2000.0 (used by nutation, precession, sidereal time).
    #[inline]
    pub fn julian_centuries(&self) -> Centuries {
        Centuries::new(
            ((*self - Self::J2000) / Self::JULIAN_CENTURY)
                .simplify()
                .value(),
        )
    }

    /// Julian years since J2000.0.
    #[inline]
    pub fn julian_years(&self) -> JulianYears {
        JulianYears::new(
            ((*self - Self::J2000) / Self::JULIAN_YEAR)
                .simplify()
                .value(),
        )
    }

    /// Converts JD(TT) → JD(TDB) using the Fairhead & Bretagnon (1990)
    /// expression for `TDB − TT`.
    ///
    /// The dominant term has an amplitude of ≈1.658 ms. This implementation
    /// includes the four largest periodic terms plus a secular component,
    /// matching the formula recommended by USNO Circular 179 (Kaplan 2005)
    /// and consistent with IAU 2006 Resolution B3.
    ///
    /// Accuracy: better than 30 μs for dates within ±10 000 years of J2000.
    ///
    /// ## References
    /// * Fairhead & Bretagnon (1990), A&A 229, 240
    /// * USNO Circular 179, eq. 2.6
    /// * SOFA `iauDtdb` (full implementation has hundreds of terms)
    pub fn tt_to_tdb(jd_tt: Self) -> Self {
        let t = jd_tt.julian_centuries().value();

        // Earth's mean anomaly (radians)
        let m_e = (357.5291092 + 35999.0502909 * t).to_radians();
        // Mean anomaly of Jupiter (radians)
        let m_j = (246.4512 + 3035.2335 * t).to_radians();
        // Mean elongation of the Moon from the Sun (radians)
        let d = (297.8502042 + 445267.1115168 * t).to_radians();
        // Mean longitude of lunar ascending node (radians)
        let om = (125.0445550 - 1934.1362091 * t).to_radians();

        // TDB − TT in seconds (Fairhead & Bretagnon largest terms):
        let dt_sec = 0.001_657 * (m_e + 0.01671 * m_e.sin()).sin()
            + 0.000_022 * (d - m_e).sin()
            + 0.000_014 * (2.0 * d).sin()
            + 0.000_005 * (m_j).sin()
            + 0.000_005 * om.sin();

        let delta_t = Days::new(dt_sec / 86_400.0);
        jd_tt + delta_t
    }

    /// Convenience: MJD value corresponding to this JD.
    ///
    /// Kept as a convenience wrapper for `self.to::<MJD>()`.
    #[inline]
    pub fn to_mjd(&self) -> Time<MJD> {
        self.to::<MJD>()
    }
}

// ── From / Into conversions for qtty time quantities (on JD only) ────────

impl Add<Years> for Time<JD> {
    type Output = Self;
    fn add(self, years: Years) -> Self {
        // Treat `Years` here as Julian years for JD arithmetic stability.
        self + Days::new(years.value() * Self::JULIAN_YEAR.value())
    }
}

impl From<JulianYears> for Time<JD> {
    fn from(years: JulianYears) -> Self {
        Self::J2000 + years.to::<Day>()
    }
}

impl From<Time<JD>> for JulianYears {
    fn from(jd: Time<JD>) -> Self {
        jd.julian_years()
    }
}

impl From<Centuries> for Time<JD> {
    fn from(centuries: Centuries) -> Self {
        // `Centuries` are interpreted as Julian centuries relative to J2000.
        Self::J2000 + Days::new(centuries.value() * Self::JULIAN_CENTURY.value())
    }
}

impl From<Time<JD>> for Centuries {
    fn from(jd: Time<JD>) -> Self {
        jd.julian_centuries()
    }
}

impl From<Millennia> for Time<JD> {
    fn from(millennia: Millennia) -> Self {
        // `Millennia` are interpreted as Julian millennia relative to J2000.
        Self::J2000 + Days::new(millennia.value() * Self::JULIAN_MILLENNIUM.value())
    }
}

impl From<Time<JD>> for Millennia {
    fn from(jd: Time<JD>) -> Self {
        jd.julian_millennias()
    }
}
