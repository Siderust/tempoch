// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scalar Serde encoding uses the scale-aware format conversion traits.
//!
//! Each serialization or deserialization captures a fresh default
//! [`crate::TimeContext`]. For civil UTC formats this selects the active UTC-TAI
//! data and rejects pre-definition dates and unrepresentable leap seconds.
//! The data snapshot is not part of the scalar payload: encoding and decoding
//! against different active data can change the physical instant. For a fixed
//! context, serialize the quantity returned by `try_raw_with(&ctx)` and decode
//! that quantity using `try_new_with(raw, &ctx)`.

use crate::format::TimeFormat;
use crate::model::scale::CoordinateScale;
use crate::model::time::Time;
use crate::period::Interval;
use crate::qtty::{Quantity, Unit};
use crate::{FormatForScale, TimeContext};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const SERIALIZED_SCALAR_NAN_ERROR: &str =
    "deserialized time scalar must not be NaN (±∞ is allowed at rest)";

impl<S: CoordinateScale, R: TimeFormat> Serialize for Time<S, R>
where
    R: FormatForScale<S>,
    Quantity<R::Unit>: Serialize,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        self.try_raw_with(&TimeContext::new())
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de, S: CoordinateScale, R: TimeFormat> Deserialize<'de> for Time<S, R>
where
    R: FormatForScale<S>,
    Quantity<R::Unit>: Deserialize<'de>,
    R::Unit: Unit,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Quantity::<R::Unit>::deserialize(deserializer)?;
        if raw.value().is_nan() {
            return Err(serde::de::Error::custom(SERIALIZED_SCALAR_NAN_ERROR));
        }
        Self::try_new_with(raw, &TimeContext::new()).map_err(serde::de::Error::custom)
    }
}

impl<T> Serialize for Interval<T>
where
    T: Copy + PartialOrd + Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Interval", 2)?;
        state.serialize_field("start", &self.start)?;
        state.serialize_field("end", &self.end)?;
        state.end()
    }
}

impl<'de, T> Deserialize<'de> for Interval<T>
where
    T: Copy + PartialOrd + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawInterval<T> {
            start: T,
            end: T,
        }

        let raw = RawInterval::<T>::deserialize(deserializer)?;
        Self::try_new(raw.start, raw.end).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{J2000s, JulianDate, JD, MJD};
    use crate::model::scale::{ContinuousScale, TAI, TDB, TT, UTC};
    use crate::qtty::{Day, Second};
    use serde::de::value::{Error as ValueError, F64Deserializer};

    #[test]
    fn encoded_time_serde_roundtrips_raw_quantity() {
        let jd = JulianDate::<TT>::new(2_451_545.25);
        let encoded = serde_json::to_string(&jd).unwrap();
        let decoded: JulianDate<TT> = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, jd);

        let seconds = Time::<TT, J2000s>::new(12.5);
        let encoded = serde_json::to_string(&seconds).unwrap();
        let decoded: Time<TT, J2000s> = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, seconds);

        assert!(serde_json::from_str::<Time<TT, JD>>("null").is_err());
    }

    fn check_utc_day_serde<F: TimeFormat<Unit = crate::qtty::unit::Day> + FormatForScale<UTC>>(
        offset: f64,
    ) {
        let ctx = TimeContext::new();
        for mjd in [51_544.0, 57_753.0, 57_754.0, 61_771.25] {
            let raw = mjd + offset;
            let time: Time<UTC, F> = serde_json::from_value(serde_json::json!(raw)).unwrap();
            let civil = Time::<UTC, MJD>::try_new_with(Day::new(mjd), &ctx).unwrap();
            assert_eq!(time.to_j2000s(), civil.to_j2000s());
            let encoded = serde_json::to_value(time).unwrap().as_f64().unwrap();
            assert!((encoded - raw).abs() < 1e-9);
            let decoded: Time<UTC, F> = serde_json::from_value(serde_json::json!(encoded)).unwrap();
            assert!((decoded.to_j2000s() - time.to_j2000s()).abs() < Second::new(50e-6));
        }

        for (raw, message) in [
            (f64::NAN, SERIALIZED_SCALAR_NAN_ERROR),
            (f64::INFINITY, "time coordinate is not usable"),
            (f64::NEG_INFINITY, "time coordinate is not usable"),
            (f64::MAX, "time coordinate is not usable"),
            (37_299.0 + offset, "date precedes 1961-01-01"),
        ] {
            let err =
                Time::<UTC, F>::deserialize(F64Deserializer::<ValueError>::new(raw)).unwrap_err();
            assert!(err.to_string().contains(message), "{raw}: {err}");
        }

        let leap_label = chrono::DateTime::parse_from_rfc3339("2016-12-31T23:59:60Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let leap = Time::<UTC>::try_from_chrono_with(leap_label, &ctx).unwrap();
        let err = serde_json::to_value(leap.reinterpret::<F>()).unwrap_err();
        assert_eq!(
            err.to_string(),
            crate::ConversionError::InvalidLeapSecond.to_string()
        );

        for value in [f64::INFINITY, f64::NEG_INFINITY] {
            let sentinel = Time::<UTC>::new(value).reinterpret::<F>();
            assert!(serde_json::to_value(sentinel).is_err());
        }

        // Scalar civil formats are no longer constrained by chrono's range.
        let distant_raw = 1e9 + offset;
        let distant = Time::<UTC, F>::try_new_with(Day::new(distant_raw), &ctx).unwrap();
        assert_eq!(
            serde_json::to_value(distant).unwrap(),
            serde_json::json!(distant_raw)
        );
        assert_eq!(
            distant.try_to_chrono_with(&ctx),
            Err(crate::ConversionError::OutOfRange)
        );

        let permissive = ctx.allow_pre_definition_utc();
        let old = Time::<UTC, F>::try_new_with(Day::new(37_299.0 + offset), &permissive).unwrap();
        assert!(serde_json::to_value(old)
            .unwrap_err()
            .to_string()
            .contains("date precedes"));

        // The existing tagged split-second wire format preserves leap instants.
        let tagged = crate::tagged::TaggedTime(leap);
        let encoded = serde_json::to_string(&tagged).unwrap();
        let decoded: crate::tagged::TaggedTime<UTC> = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, tagged);
        assert!(decoded.0.is_leap_second_with(&TimeContext::new()));
    }

    #[test]
    fn utc_jd_mjd_serde_uses_civil_coordinates_and_reports_conversion_errors() {
        check_utc_day_serde::<JD>(2_400_000.5);
        check_utc_day_serde::<MJD>(0.0);
    }

    fn check_continuous_day_serde<S, F>(raw: f64)
    where
        S: ContinuousScale,
        F: TimeFormat<Unit = crate::qtty::unit::Day> + FormatForScale<S>,
    {
        let time: Time<S, F> = serde_json::from_value(serde_json::json!(raw)).unwrap();
        assert_eq!(serde_json::to_value(time).unwrap(), serde_json::json!(raw));
        for value in [f64::INFINITY, f64::NEG_INFINITY] {
            let time =
                Time::<S, F>::deserialize(F64Deserializer::<ValueError>::new(value)).unwrap();
            assert_eq!(
                time.try_raw_with(&TimeContext::new()).unwrap().value(),
                value
            );
            // JSON cannot represent infinity, matching the existing serializer behavior.
            assert_eq!(serde_json::to_value(time).unwrap(), serde_json::Value::Null);
        }
        assert!(Time::<S, F>::deserialize(F64Deserializer::<ValueError>::new(f64::NAN)).is_err());
    }

    #[test]
    fn continuous_jd_mjd_serde_retains_scalar_and_sentinel_behavior() {
        check_continuous_day_serde::<TT, JD>(2_451_545.25);
        check_continuous_day_serde::<TT, MJD>(51_544.75);
        check_continuous_day_serde::<TAI, JD>(2_451_545.25);
        check_continuous_day_serde::<TAI, MJD>(51_544.75);
        check_continuous_day_serde::<TDB, JD>(2_451_545.25);
        check_continuous_day_serde::<TDB, MJD>(51_544.75);
    }
}
