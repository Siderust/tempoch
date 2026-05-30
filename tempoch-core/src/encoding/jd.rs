use crate::foundation::constats::J2000_JD_TT_DAY;
use qtty::Day;

/// Julian Day TT → Julian centuries since J2000 TT (dimensionless).
#[inline]
pub(crate) fn jd_to_julian_centuries(jd: Day) -> f64 {
    (jd - J2000_JD_TT_DAY)
        .to::<qtty::unit::JulianCentury>()
        .value()
}
