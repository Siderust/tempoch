// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed epoch and offset constants.
//!
//! These values are exposed as raw `qtty` quantities so callers can pass them
//! directly to `Time::<A>::from_julian_days`, `from_modified_julian_days`, etc.

use qtty::{Day, Second};

pub use crate::delta_t::DELTA_T_PREDICTION_HORIZON_MJD;

/// J2000 epoch as JD(TT) = 2_451_545.0.
pub const J2000_JD_TT: Day = Day::new(2_451_545.0);

/// Offset between the Julian Day and Modified Julian Day counts.
///
/// `MJD = JD - JD_MINUS_MJD`.
pub const JD_MINUS_MJD: Day = Day::new(2_400_000.5);

/// Exact `TT - TAI` offset (32.184 s).
pub const TT_MINUS_TAI: Second = Second::new(32.184);

/// Unix epoch as a Julian Day on the UTC axis: 1970-01-01T00:00:00 UTC.
pub const UNIX_EPOCH_JD: Day = Day::new(2_440_587.5);

/// Unix epoch as a Modified Julian Day on the UTC axis.
pub const UNIX_EPOCH_MJD: Day = Day::new(40_587.0);

/// IAU 2000 B1.9 reference epoch `T0` as JD(TT).
pub const IAU_TIME_EPOCH_T0_JD: Day = Day::new(2_443_144.500_372_5);

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
pub const TDB_TT_MODEL_HIGH_ACCURACY_START_JD: Day = Day::new(2_305_447.5);

/// End of the interval where the built-in TT↔TDB truncated series achieves
/// about 10 microseconds accuracy relative to numerical integration.
///
/// See [`TDB_TT_MODEL_HIGH_ACCURACY_START_JD`] for the full accuracy breakdown.
/// This constant corresponds approximately to 2200-01-01 TT.
pub const TDB_TT_MODEL_HIGH_ACCURACY_END_JD: Day = Day::new(2_524_598.5);

/// GPS epoch expressed as TAI seconds since J2000 TT on the TAI axis.
///
/// The storage convention is `(JD_TAI(P) − J2000_JD_TT) × 86400`. For the GPS
/// epoch, `JD_UTC = 2_444_244.5` and `TAI − UTC = 19 s` (exact), giving:
///
///   `(44_244.0 − 51_544.5) × 86400 + 19 = −630_763_181`.
pub const GPS_EPOCH_TAI: Second = Second::new(-630_763_181.0);

/// First MJD covered by the compiled UTC-TAI segment table.
///
/// This corresponds to 1961-01-01. UTC was defined starting from this date.
/// For queries before this boundary, `Time<UTC>` conversions silently
/// extrapolate the first table segment backwards. The extrapolated offset is
/// internally consistent (round-trips close) but is not a historically defined
/// UTC-TAI value; no standard UTC existed before 1961.
///
/// Call sites that require historically accurate UTC values should guard
/// against this boundary:
/// ```no_run
/// use tempoch_core::constats::UTC_DEFINED_FROM_MJD;
/// // reject MJDs below UTC_DEFINED_FROM_MJD
/// ```
pub const UTC_DEFINED_FROM_MJD: Day = Day::new(37_300.0);

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
        assert!((UNIX_EPOCH_JD - JD_MINUS_MJD - UNIX_EPOCH_MJD).abs() < Day::new(1e-15));
    }

    #[test]
    fn j2000_reference_values_match_known_offsets() {
        assert!((J2000_JD_TT - JD_MINUS_MJD - Day::new(51_544.5)).abs() < Day::new(1e-12));
        assert!((TT_MINUS_TAI - Second::new(32.184)).abs() < Second::new(1e-12));
        assert!((UTC_DEFINED_FROM_MJD - Day::new(37_300.0)).abs() < Day::new(1e-12));
    }

    #[test]
    fn high_accuracy_model_interval_is_ordered() {
        assert!(TDB_TT_MODEL_HIGH_ACCURACY_END_JD > TDB_TT_MODEL_HIGH_ACCURACY_START_JD);
        assert!(GPS_EPOCH_TAI.is_finite());
        assert!(DELTA_T_PREDICTION_HORIZON_MJD.is_finite());
    }
}
