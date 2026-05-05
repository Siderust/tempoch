// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use crate::interval::Interval;
use crate::representation::{EncodedTime, TimeRepresentation};
use crate::scale::Scale;
use crate::time::Time;
use qtty::{Quantity, Second, Unit};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const NONFINITE_TIME_VALUE_ERROR: &str = "time value must be finite (not NaN or infinity)";

impl<S: Scale, R: TimeRepresentation> Serialize for EncodedTime<S, R>
where
    Quantity<R::Unit>: Serialize,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        self.raw().serialize(serializer)
    }
}

impl<'de, S: Scale, R: TimeRepresentation> Deserialize<'de> for EncodedTime<S, R>
where
    Quantity<R::Unit>: Deserialize<'de>,
    R::Unit: Unit,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Quantity::<R::Unit>::deserialize(deserializer)?;
        if !raw.is_finite() {
            return Err(serde::de::Error::custom(NONFINITE_TIME_VALUE_ERROR));
        }
        Ok(Self::new_unchecked(raw))
    }
}


impl<S: Scale> Serialize for Time<S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut state = serializer.serialize_struct("Time", 2)?;
        let (hi, lo) = self.raw_seconds_pair();
        state.serialize_field("hi", &hi.value())?;
        state.serialize_field("lo", &lo.value())?;
        state.end()
    }
}

impl<'de, S: Scale> Deserialize<'de> for Time<S> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawTime {
            hi: f64,
            lo: f64,
        }

        let raw = RawTime::deserialize(deserializer)?;
        if !(raw.hi.is_finite() && raw.lo.is_finite()) {
            return Err(serde::de::Error::custom(NONFINITE_TIME_VALUE_ERROR));
        }
        Time::try_new(Second::new(raw.hi), Second::new(raw.lo)).map_err(serde::de::Error::custom)
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
