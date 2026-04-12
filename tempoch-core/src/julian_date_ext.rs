// SPDX-License-Identifier: AGPL-3.0-only
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
    pub const JULIAN_YEAR: Day = Day::new(365.25);

    /// One Julian century expressed in days.
    pub const JULIAN_CENTURY: Day = Day::new(36_525.0);

    /// One Julian millennium expressed in days.
    pub const JULIAN_MILLENNIUM: Day = Day::new(365_250.0);

    /// Julian millennia since J2000.0 (used by VSOP87).
    #[inline]
    pub fn julian_millennia(&self) -> Millennium {
        Millennium::new(self.julian_years().value() / 1000.0)
    }

    /// Julian centuries since J2000.0 (used by nutation, precession, sidereal time).
    #[inline]
    pub fn julian_centuries(&self) -> JulianCentury {
        (*self - Self::J2000).to::<qtty::unit::JulianCentury>()
    }

    /// Julian years since J2000.0.
    #[inline]
    pub fn julian_years(&self) -> JulianYear {
        (*self - Self::J2000).to::<qtty::unit::JulianYear>()
    }

    /// Converts JD(TT) → JD(TDB) using the Fairhead & Bretagnon (1990)
    /// expression for `TDB − TT`.
    ///
    /// The dominant term has an amplitude of ≈1.658 ms. This implementation
    /// includes the four largest periodic terms (all harmonic sine terms;
    /// no secular polynomial component), matching the formula recommended by
    /// USNO Circular 179 (Kaplan 2005) and consistent with IAU 2006
    /// Resolution B3.
    ///
    /// **Accuracy note:** The formula is accurate to better than 30 μs for
    /// dates within ±10 000 years of J2000. However, the `f64` storage floor
    /// at J2000 is ≈40 μs (one ULP ≈ 4.66 × 10⁻¹⁰ d), so the effective
    /// combined precision is **≈40 μs**, not better.
    ///
    /// ## References
    /// * Fairhead & Bretagnon (1990), A&A 229, 240
    /// * USNO Circular 179, eq. 2.6
    /// * SOFA `iauDtdb` (full implementation has hundreds of terms)
    pub fn tt_to_tdb(jd_tt: Self) -> Self {
        jd_tt + super::scales::tdb_minus_tt_days(jd_tt.quantity())
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

impl Add<Year> for Time<JD> {
    type Output = Self;
    fn add(self, years: Year) -> Self {
        // Treat `Year` here as Julian years for JD arithmetic stability.
        self + Day::new(years.value() * Self::JULIAN_YEAR.value())
    }
}

impl From<JulianYear> for Time<JD> {
    fn from(years: JulianYear) -> Self {
        Self::J2000 + years.to::<qtty::unit::Day>()
    }
}

impl From<Time<JD>> for JulianYear {
    fn from(jd: Time<JD>) -> Self {
        jd.julian_years()
    }
}

impl From<JulianCentury> for Time<JD> {
    fn from(centuries: JulianCentury) -> Self {
        Self::J2000 + centuries.to::<qtty::unit::Day>()
    }
}

impl From<Time<JD>> for JulianCentury {
    fn from(jd: Time<JD>) -> Self {
        jd.julian_centuries()
    }
}

impl From<Millennium> for Time<JD> {
    fn from(millennia: Millennium) -> Self {
        // `Millennium` are interpreted as Julian millennia relative to J2000.
        Self::J2000 + Day::new(millennia.value() * Self::JULIAN_MILLENNIUM.value())
    }
}

impl From<Time<JD>> for Millennium {
    fn from(jd: Time<JD>) -> Self {
        jd.julian_millennia()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Time, JD, MJD};

    #[test]
    fn to_mjd_convenience_matches_to_generic() {
        // to_mjd() is a convenience wrapper for .to::<MJD>().
        let jd = Time::<JD>::J2000;
        let via_convenience = jd.to_mjd();
        let via_generic = jd.to::<MJD>();
        assert_eq!(via_convenience.value(), via_generic.value());
    }
}
