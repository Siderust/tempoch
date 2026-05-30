use crate::foundation::constats::{unix_epoch_mjd_day, JD_MINUS_MJD, UNIX_EPOCH_JD_DAY};
use qtty::unit::{Day as DayUnit, Second as SecondUnit};
use qtty::{Day, Second};

/// Julian Day → Modified Julian Day.
#[inline]
pub(crate) fn jd_to_mjd(jd: Day) -> Day {
    jd - JD_MINUS_MJD
}

/// UTC MJD → seconds since Unix epoch (1970-01-01).
#[inline]
pub(crate) fn mjd_to_unix_seconds(mjd: Day) -> Second {
    (mjd - unix_epoch_mjd_day()).to::<SecondUnit>()
}

/// Seconds since Unix epoch → UTC MJD.
#[inline]
pub(crate) fn unix_seconds_to_mjd(seconds: Second) -> Day {
    unix_epoch_mjd_day() + seconds.to::<DayUnit>()
}

/// Seconds since Unix epoch → Julian Day (UTC axis).
#[inline]
pub(crate) fn unix_seconds_to_jd(seconds: Second) -> Day {
    UNIX_EPOCH_JD_DAY + seconds.to::<DayUnit>()
}
