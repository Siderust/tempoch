// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

#![cfg(feature = "serde")]

use crate::format::{DayCount, Format, GpsSecs, J2000s, JD, MJD, UnixSecs};
use crate::interval::Interval;
use crate::scale::Scale;
use crate::time::Time;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const NONFINITE_TIME_VALUE_ERROR: &str = "time value must be finite (not NaN or infinity)";

#[allow(private_bounds)]
pub(crate) trait SerdeFormat: Format {
    fn validate_serde_value(value: &Self::Storage) -> Result<(), &'static str>;
}

macro_rules! impl_serde_format_finite {
    ($($format:ty),+ $(,)?) => {
        $(
            impl SerdeFormat for $format {
                #[inline]
                fn validate_serde_value(value: &Self::Storage) -> Result<(), &'static str> {
                    if value.is_finite() {
                        Ok(())
                    } else {
                        Err(NONFINITE_TIME_VALUE_ERROR)
                    }
                }
            }
        )+
    };
}

macro_rules! impl_serde_format_passthrough {
    ($($format:ty),+ $(,)?) => {
        $(
            impl SerdeFormat for $format {
                #[inline]
                fn validate_serde_value(_value: &Self::Storage) -> Result<(), &'static str> {
                    Ok(())
                }
            }
        )+
    };
}

impl_serde_format_finite!(J2000s, JD, MJD, GpsSecs);
impl_serde_format_passthrough!(UnixSecs, DayCount);

#[allow(private_bounds)]
impl<S: Scale, F> Serialize for Time<S, F>
where
    F: Format + SerdeFormat,
    F::Storage: Serialize,
{
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        (*self).value().serialize(serializer)
    }
}

#[allow(private_bounds)]
impl<'de, S: Scale, F> Deserialize<'de> for Time<S, F>
where
    F: Format + SerdeFormat,
    F::Storage: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = F::Storage::deserialize(deserializer)?;
        <F as SerdeFormat>::validate_serde_value(&value).map_err(serde::de::Error::custom)?;
        Ok(Self::new(value))
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
