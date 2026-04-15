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

/// GPS epoch expressed as TAI seconds since J2000 TT on the TAI axis.
///
/// The storage convention is `(JD_TAI(P) − J2000_JD_TT) × 86400`. For the GPS
/// epoch, `JD_UTC = 2_444_244.5` and `TAI − UTC = 19 s` (exact), giving:
///
///   `(44_244.0 − 51_544.5) × 86400 + 19 = −630_763_181`.
pub const GPS_EPOCH_TAI: Second = Second::new(-630_763_181.0);

/// One Julian century in days (36 525 d), used for the Fairhead–Bretagnon
/// parameter.
pub(crate) const DAYS_PER_JC: Day = Day::new(36_525.0);

pub(crate) const UTC_INTERVAL_EPS: Day = Day::new(1e-15);
pub(crate) const L_G: f64 = 6.969_290_134e-10;
pub(crate) const L_B: f64 = 1.550_519_768e-8;
pub(crate) const TDB0: Second = Second::new(-6.55e-5);
