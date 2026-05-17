// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use crate::format::TimeFormat;
use crate::model::scale::CoordinateScale;
use crate::model::time::Time;
use crate::period::Interval;
use crate::InfallibleFormatForScale;
use qtty::{Quantity, Unit};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const SERIALIZED_SCALAR_NAN_ERROR: &str =
    "deserialized time scalar must not be NaN (±∞ is allowed at rest)";

impl<S: CoordinateScale, R: TimeFormat> Serialize for Time<S, R>
where
    R: InfallibleFormatForScale<S>,
    Quantity<R::Unit>: Serialize,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        self.raw().serialize(serializer)
    }
}

impl<'de, S: CoordinateScale, R: TimeFormat> Deserialize<'de> for Time<S, R>
where
    R: InfallibleFormatForScale<S>,
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
        Ok(R::into_time(raw))
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
    use crate::format::{J2000s, JulianDate, JD};
    use crate::model::scale::TT;

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
}
