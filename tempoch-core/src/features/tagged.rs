// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Tagged serde wire formats for interchange use-cases.
//!
//! The crate-level `Serialize`/`Deserialize` impls for [`crate::Time`] and
//! [`crate::Period`] intentionally keep the scale out of band. This module
//! provides explicit scale-tagged wrappers when the payload itself must carry
//! scale identity. Both wrappers accept a format tag `F`, which is zero-cost;
//! the wire fields always carry canonical split J2000 seconds.

use crate::format::{J2000s, TimeFormat};
use crate::model::scale::{CoordinateScale, Scale};
use crate::model::time::Time;
use crate::period::Interval;
use crate::qtty::Second;
use alloc::format;
use alloc::string::String;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const TAGGED_SCALAR_NAN_ERROR: &str = "tagged time scalars must not be NaN (±∞ is allowed at rest)";

/// Scale-tagged interchange encoding with canonical compensated J2000 seconds.
///
/// `F` is a phantom format tag and does not change the `hi`/`lo` wire fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaggedTime<S: Scale, F: TimeFormat = J2000s>(pub Time<S, F>);

impl<S: Scale, F: TimeFormat> From<Time<S, F>> for TaggedTime<S, F> {
    #[inline]
    fn from(value: Time<S, F>) -> Self {
        Self(value)
    }
}

impl<S: Scale, F: TimeFormat> From<TaggedTime<S, F>> for Time<S, F> {
    #[inline]
    fn from(value: TaggedTime<S, F>) -> Self {
        value.0
    }
}

impl<S: Scale, F: TimeFormat> Serialize for TaggedTime<S, F> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut state = serializer.serialize_struct("TaggedTime", 3)?;
        let (hi, lo) = self.0.raw_seconds_pair();
        state.serialize_field("scale", S::NAME)?;
        state.serialize_field("hi", &hi.value())?;
        state.serialize_field("lo", &lo.value())?;
        state.end()
    }
}

impl<'de, S: Scale + CoordinateScale, F: TimeFormat> Deserialize<'de> for TaggedTime<S, F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawTaggedTime {
            scale: String,
            hi: f64,
            lo: f64,
        }

        let raw = RawTaggedTime::deserialize(deserializer)?;
        if raw.scale != S::NAME {
            return Err(serde::de::Error::custom(format!(
                "expected scale {}, got {}",
                S::NAME,
                raw.scale
            )));
        }
        if raw.hi.is_nan() || raw.lo.is_nan() {
            return Err(serde::de::Error::custom(TAGGED_SCALAR_NAN_ERROR));
        }
        Ok(Self(
            Time::<S, J2000s>::try_from_raw_j2000_seconds_split(
                Second::new(raw.hi),
                Second::new(raw.lo),
            )
            .map_err(serde::de::Error::custom)?
            .reinterpret::<F>(),
        ))
    }
}

/// Tagged half-open interval. `F` is a phantom view; the on-wire representation
/// always holds canonical split J2000 seconds, not `F`-encoded scalars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TaggedPeriod<S: Scale, F: TimeFormat = J2000s>(pub Interval<Time<S, F>>);

impl<S: Scale, F: TimeFormat> From<Interval<Time<S, F>>> for TaggedPeriod<S, F> {
    #[inline]
    fn from(value: Interval<Time<S, F>>) -> Self {
        Self(value)
    }
}

impl<S: Scale, F: TimeFormat> From<TaggedPeriod<S, F>> for Interval<Time<S, F>> {
    #[inline]
    fn from(value: TaggedPeriod<S, F>) -> Self {
        value.0
    }
}

impl<S: Scale, F: TimeFormat> Serialize for TaggedPeriod<S, F> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut state = serializer.serialize_struct("TaggedPeriod", 3)?;
        state.serialize_field("scale", S::NAME)?;
        state.serialize_field("start", &TaggedTime(self.0.start))?;
        state.serialize_field("end", &TaggedTime(self.0.end))?;
        state.end()
    }
}

impl<'de, S: Scale + CoordinateScale, F: TimeFormat> Deserialize<'de> for TaggedPeriod<S, F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawTaggedTime {
            scale: String,
            hi: f64,
            lo: f64,
        }

        #[derive(Deserialize)]
        struct RawTaggedPeriod {
            scale: String,
            start: RawTaggedTime,
            end: RawTaggedTime,
        }

        let decode_time = |raw: RawTaggedTime| -> Result<Time<S, F>, D::Error> {
            if raw.scale != S::NAME {
                return Err(serde::de::Error::custom(format!(
                    "expected scale {}, got {}",
                    S::NAME,
                    raw.scale
                )));
            }
            if raw.hi.is_nan() || raw.lo.is_nan() {
                return Err(serde::de::Error::custom(TAGGED_SCALAR_NAN_ERROR));
            }
            Time::<S, J2000s>::try_from_raw_j2000_seconds_split(
                Second::new(raw.hi),
                Second::new(raw.lo),
            )
            .map(Time::reinterpret::<F>)
            .map_err(serde::de::Error::custom)
        };

        let raw = RawTaggedPeriod::deserialize(deserializer)?;
        if raw.scale != S::NAME {
            return Err(serde::de::Error::custom(format!(
                "expected scale {}, got {}",
                S::NAME,
                raw.scale
            )));
        }
        Ok(Self(
            Interval::try_new(decode_time(raw.start)?, decode_time(raw.end)?)
                .map_err(serde::de::Error::custom)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qtty::Second;
    use crate::{Period, TT, UTC};
    use serde_json::json;

    #[test]
    fn tagged_time_roundtrips_with_scale_field() {
        let tt = Time::<TT>::from_raw_j2000_seconds(Second::new(42.5)).unwrap();
        let payload = TaggedTime(tt);
        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            json!({"scale": "TT", "hi": 42.5, "lo": 0.0})
        );
        assert_eq!(
            serde_json::from_value::<TaggedTime<TT>>(json!({
                "scale": "TT",
                "hi": 42.5,
                "lo": 0.0
            }))
            .unwrap(),
            TaggedTime(tt)
        );
    }

    #[test]
    fn tagged_time_rejects_scale_mismatch() {
        let err = serde_json::from_value::<TaggedTime<UTC>>(json!({
            "scale": "TT",
            "hi": 0.0,
            "lo": 0.0
        }))
        .unwrap_err();
        assert!(err.to_string().contains("expected scale UTC"));
    }

    #[test]
    fn tagged_period_roundtrips() {
        let period = Period::<TT>::new(
            Time::<TT>::from_raw_j2000_seconds(Second::new(1.25)).unwrap(),
            Time::<TT>::from_raw_j2000_seconds(Second::new(2.5)).unwrap(),
        );
        let payload = TaggedPeriod(period);
        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            json!({
                "scale": "TT",
                "start": {"scale": "TT", "hi": 1.25, "lo": 0.0},
                "end": {"scale": "TT", "hi": 2.5, "lo": 0.0}
            })
        );
        assert_eq!(
            serde_json::from_value::<TaggedPeriod<TT>>(json!({
                "scale": "TT",
                "start": {"scale": "TT", "hi": 1.25, "lo": 0.0},
                "end": {"scale": "TT", "hi": 2.5, "lo": 0.0}
            }))
            .unwrap(),
            TaggedPeriod(period)
        );
    }

    #[test]
    fn tagged_from_impls_preserve_inner_values() {
        let tt = Time::<TT>::from_raw_j2000_seconds(Second::new(3.5)).unwrap();
        let tagged_time: TaggedTime<TT> = tt.into();
        let time: Time<TT> = tagged_time.into();
        assert_eq!(time, tt);

        let period = Period::<TT>::new(
            Time::<TT>::from_raw_j2000_seconds(Second::new(1.0)).unwrap(),
            Time::<TT>::from_raw_j2000_seconds(Second::new(2.0)).unwrap(),
        );
        let tagged_period: TaggedPeriod<TT> = period.into();
        let decoded_period: Period<TT> = tagged_period.into();
        assert_eq!(decoded_period, period);
    }

    #[test]
    fn tagged_period_rejects_outer_and_nested_scale_mismatches() {
        let outer_err = serde_json::from_value::<TaggedPeriod<TT>>(json!({
            "scale": "UTC",
            "start": {"scale": "TT", "hi": 1.0, "lo": 0.0},
            "end": {"scale": "TT", "hi": 2.0, "lo": 0.0}
        }))
        .unwrap_err();
        assert!(outer_err.to_string().contains("expected scale TT"));

        let nested_err = serde_json::from_value::<TaggedPeriod<TT>>(json!({
            "scale": "TT",
            "start": {"scale": "UTC", "hi": 1.0, "lo": 0.0},
            "end": {"scale": "TT", "hi": 2.0, "lo": 0.0}
        }))
        .unwrap_err();
        assert!(nested_err.to_string().contains("expected scale TT"));
    }

    #[test]
    fn tagged_period_rejects_reversed_endpoints() {
        let err = serde_json::from_value::<TaggedPeriod<TT>>(json!({
            "scale": "TT",
            "start": {"scale": "TT", "hi": 2.0, "lo": 0.0},
            "end": {"scale": "TT", "hi": 1.0, "lo": 0.0}
        }))
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("interval start must not be after end"));
    }
}
