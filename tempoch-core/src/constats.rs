// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed epoch and offset constants.
//!
//! Coordinate-style constants are exposed as [`Coord<S, F>`] values that
//! carry both their **scale** (TT, TAI, UTC, UT1, …) and their **format**
//! (JD, MJD, J2000 seconds, …) at the type level. This prevents cross-axis
//! misuse such as feeding a UTC-axis Julian Date to a TT-tagged
//! [`JulianDate<TT>`] constructor.
//!
//! Pure SI-second offsets between scales (e.g. [`TT_MINUS_TAI`]) remain
//! plain [`qtty::Second`] values: they are scale-independent durations
//! used as algebraic offsets, not instants.
//!
//! [`Coord<S, F>`]: crate::Coord
//! [`JulianDate<TT>`]: crate::JulianDate

use qtty::{Day, Second};

/// Days in a Julian year (365.25 d).
pub const JULIAN_YEAR_DAYS: Day = Day::new(365.25);

use crate::coord::Coord;
use crate::format::{J2000s, JD, MJD};
use crate::scale::{TAI, TT, UTC};

pub use crate::delta_t::DELTA_T_PREDICTION_HORIZON_MJD;

/// J2000 epoch as a JD on the TT axis (`JD 2 451 545.0 TT`).
pub const J2000_JD_TT: Coord<TT, JD> = Coord::from_raw_unchecked(Day::new(2_451_545.0));

/// Offset between Julian Day and Modified Julian Day counts.
///
/// `MJD = JD - JD_MINUS_MJD`. Kept crate-private: external callers should
/// use the typed conversions on [`Coord<S, JD>`] / [`Coord<S, MJD>`]
/// instead of doing the offset arithmetic by hand.
pub(crate) const JD_MINUS_MJD: Day = Day::new(2_400_000.5);

/// Exact `TT - TAI` offset (32.184 s).
///
/// This is a pure SI-second offset between two coordinate scales, not an
/// instant; it is intentionally kept as a [`qtty::Second`] for algebraic
/// use in scale conversions.
pub const TT_MINUS_TAI: Second = Second::new(32.184);

/// Unix epoch as a JD on the UTC axis: `1970-01-01T00:00:00 UTC`.
pub const UNIX_EPOCH_JD: Coord<UTC, JD> = Coord::from_raw_unchecked(Day::new(2_440_587.5));

/// Unix epoch as an MJD on the UTC axis.
pub const UNIX_EPOCH_MJD: Coord<UTC, MJD> = Coord::from_raw_unchecked(Day::new(40_587.0));

/// GPS epoch as a JD on the UTC axis: `1980-01-06T00:00:00 UTC`.
pub const GPS_EPOCH_JD_UTC: Coord<UTC, JD> = Coord::from_raw_unchecked(Day::new(2_444_244.5));

/// Exact `TAI - UTC` offset at the GPS epoch.
///
/// Like [`TT_MINUS_TAI`], this is a pure SI-second offset and stays as a
/// bare [`qtty::Second`].
pub const GPS_EPOCH_TAI_MINUS_UTC: Second = Second::new(19.0);

/// GPS epoch expressed as a JD on the TAI axis.
///
/// At the GPS epoch, `TAI - UTC = 19 s` exactly, so this is
/// `GPS_EPOCH_JD_UTC + 19 s` converted to Julian days, but on the TAI axis.
pub const GPS_EPOCH_JD_TAI: Coord<TAI, JD> = Coord::from_raw_unchecked(
    GPS_EPOCH_JD_UTC
        .raw()
        .const_add(GPS_EPOCH_TAI_MINUS_UTC.to_const::<qtty::unit::Day>()),
);

/// IAU 2000 B1.9 reference epoch `T0` as a JD on the TT axis.
pub const IAU_TIME_EPOCH_T0_JD: Coord<TT, JD> =
    Coord::from_raw_unchecked(Day::new(2_443_144.500_372_5));

/// Start of the interval where the built-in TT↔TDB truncated series achieves
/// about 10 microseconds accuracy relative to numerical integration.
///
/// The seven-term Fairhead-Bretagnon truncation (USNO Circular 179, Eq. 2.27)
/// has two distinct error budgets:
///
/// - **~2 µs** relative to the full Fairhead-Bretagnon (1990) series (series
///   truncation error only).
/// - **~10 µs** relative to JPL numerical integration (full series + modeling
///   error combined).
///
/// The **end-to-end** accuracy ceiling is therefore **~10 µs**. These bounds
/// apply within the 1600-01-01 to 2200-01-01 TT interval. This constant marks
/// the start of that interval, corresponding approximately to 1600-01-01 TT.
pub const TDB_TT_MODEL_HIGH_ACCURACY_START_JD: Coord<TT, JD> =
    Coord::from_raw_unchecked(Day::new(2_305_447.5));

/// End of the interval where the built-in TT↔TDB truncated series achieves
/// about 10 microseconds accuracy relative to numerical integration.
///
/// See [`TDB_TT_MODEL_HIGH_ACCURACY_START_JD`] for the full accuracy breakdown.
/// This constant corresponds approximately to 2200-01-01 TT.
pub const TDB_TT_MODEL_HIGH_ACCURACY_END_JD: Coord<TT, JD> =
    Coord::from_raw_unchecked(Day::new(2_524_598.5));

/// GPS epoch expressed as J2000-second offset on the TAI axis.
///
/// The storage convention is `(JD_TAI(P) − J2000_JD_TT) × 86400`. For the GPS
/// epoch, `JD_UTC = GPS_EPOCH_JD_UTC` and `TAI − UTC = 19 s` (exact), giving:
///
///   `(44_244.0 − 51_544.5) × 86400 + 19 = −630_763_181`.
pub const GPS_EPOCH_TAI: Coord<TAI, J2000s> =
    Coord::from_raw_unchecked(Second::new(-630_763_181.0));

/// First MJD covered by the compiled UTC-TAI segment table, on the UTC axis.
///
/// This corresponds to 1961-01-01. UTC was defined starting from this date.
/// For queries before this boundary, `Time<UTC>` conversions return
/// [`crate::ConversionError::UtcBeforeDefinition`] by default. Back-extrapolation
/// of the first segment can be enabled by building the conversion context with
/// [`crate::TimeContext::allow_pre_definition_utc`]. The extrapolated offset is
/// internally consistent (round-trips close) but is not a historically defined
/// UTC-TAI value; no standard UTC existed before 1961.
pub const UTC_DEFINED_FROM_MJD: Coord<UTC, MJD> = Coord::from_raw_unchecked(Day::new(37_300.0));

/// One Julian century in days (36 525 d), used for the Fairhead–Bretagnon
/// parameter.
pub(crate) const DAYS_PER_JC: Day = Day::new(36_525.0);

pub(crate) const UTC_INTERVAL_EPS: Day = Day::new(1e-15);
pub(crate) const L_G: f64 = 6.969_290_134e-10;
pub(crate) const L_B: f64 = 1.550_519_768e-8;
pub(crate) const TDB0: Second = Second::new(-6.55e-5);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_jd_and_mjd_constants_are_consistent() {
        assert!(
            (UNIX_EPOCH_JD.raw() - JD_MINUS_MJD - UNIX_EPOCH_MJD.raw()).abs() < Day::new(1e-15)
        );
    }

    #[test]
    fn j2000_reference_values_match_known_offsets() {
        assert!((J2000_JD_TT.raw() - JD_MINUS_MJD - Day::new(51_544.5)).abs() < Day::new(1e-12));
        assert!((TT_MINUS_TAI - Second::new(32.184)).abs() < Second::new(1e-12));
        assert!((UTC_DEFINED_FROM_MJD.raw() - Day::new(37_300.0)).abs() < Day::new(1e-12));
        assert!((GPS_EPOCH_JD_UTC.raw() - Day::new(2_444_244.5)).abs() < Day::new(1e-12));
        assert!((GPS_EPOCH_TAI_MINUS_UTC - Second::new(19.0)).abs() < Second::new(1e-12));
        assert!(
            (GPS_EPOCH_JD_TAI.raw()
                - GPS_EPOCH_JD_UTC.raw()
                - GPS_EPOCH_TAI_MINUS_UTC.to::<qtty::unit::Day>())
            .abs()
                < Day::new(1e-9)
        );
    }

    #[test]
    fn high_accuracy_model_interval_is_ordered() {
        assert!(TDB_TT_MODEL_HIGH_ACCURACY_END_JD > TDB_TT_MODEL_HIGH_ACCURACY_START_JD);
        assert!(GPS_EPOCH_TAI.raw().is_finite());
        assert!(DELTA_T_PREDICTION_HORIZON_MJD.raw().is_finite());
    }
}
