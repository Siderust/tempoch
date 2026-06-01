// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! GNSS week-number decomposition exposed over the C ABI.
//!
//! Mirrors [`tempoch::GnssWeek`] and the `Time::<S>::to_gnss_week` /
//! `Time::<S>::from_gnss_week` conversions, which are defined only for the GNSS
//! coordinate scales (`GPST`, `GST`, `BDT`, `QZSST`).  Any other scale tag is
//! rejected with [`TempochStatus::InvalidScaleId`].

use crate::catch_panic;
use crate::error::TempochStatus;
use crate::typed::{status_from_conversion, TempochScaleTag, TempochTime};
use qtty::Second;
use tempoch::ConversionError;
use tempoch::{GnssWeek, Time, BDT, GPST, GST, QZSST};

/// Decomposed GNSS week-number form since the constellation's defined epoch.
///
/// Mirrors [`tempoch::GnssWeek`]. The week number is *full* (no rollover);
/// `seconds_of_week` lies in `[0, 604_800)` and `subsecond_nanos` in
/// `[0, 1_000_000_000)`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempochGnssWeek {
    /// Full week number since the constellation's epoch (no rollover applied).
    pub week: u32,
    /// Seconds since the start of `week`, in `[0, 604_800)`.
    pub seconds_of_week: u32,
    /// Subsecond nanoseconds remainder, in `[0, 1_000_000_000)`.
    pub subsecond_nanos: u32,
}

macro_rules! with_gnss_scale {
    ($tag:expr, $Scale:ident, $body:block) => {
        match $tag {
            TempochScaleTag::GPST => {
                type $Scale = GPST;
                $body
            }
            TempochScaleTag::GST => {
                type $Scale = GST;
                $body
            }
            TempochScaleTag::BDT => {
                type $Scale = BDT;
                $body
            }
            TempochScaleTag::QZSST => {
                type $Scale = QZSST;
                $body
            }
            _ => return TempochStatus::InvalidScaleId,
        }
    };
}

/// Decompose a GNSS-scale split instant into its week-number form.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochGnssWeek`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_to_gnss_week(
    value: TempochTime,
    scale: i32,
    out: *mut TempochGnssWeek,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let Some(scale) = TempochScaleTag::from_raw(scale) else {
            return TempochStatus::InvalidScaleId;
        };
        let result = with_gnss_scale!(scale, S, {
            Time::<S>::try_from_raw_j2000_seconds_split(
                Second::new(value.hi_seconds),
                Second::new(value.lo_seconds),
            )
            .and_then(|t| t.to_gnss_week())
        });
        match result {
            Ok(gw) => {
                let encoded = TempochGnssWeek {
                    week: gw.week.value(),
                    seconds_of_week: gw.seconds_of_week.value(),
                    subsecond_nanos: gw.subsecond_nanos.value(),
                };
                // SAFETY: `out` was checked for null and the safety contract
                // requires it to point to writable `TempochGnssWeek` storage.
                unsafe { *out = encoded };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

/// Build a GNSS-scale split instant from a week-number decomposition.
///
/// # Safety
/// `out` must be a valid, writable pointer to `TempochTime`.
#[no_mangle]
pub unsafe extern "C" fn tempoch_time_from_gnss_week(
    value: TempochGnssWeek,
    scale: i32,
    out: *mut TempochTime,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let Some(scale) = TempochScaleTag::from_raw(scale) else {
            return TempochStatus::InvalidScaleId;
        };
        let gw = match GnssWeek::new(
            qtty::u32::Week::new(value.week),
            qtty::u32::Second::new(value.seconds_of_week),
            qtty::u32::Nanosecond::new(value.subsecond_nanos),
        ) {
            Ok(gw) => gw,
            Err(err) => return status_from_conversion(err),
        };
        let result: Result<TempochTime, ConversionError> = with_gnss_scale!(scale, S, {
            Time::<S>::from_gnss_week(gw).map(|time| {
                let (hi, lo) = time.raw_seconds_pair();
                TempochTime {
                    hi_seconds: hi.value(),
                    lo_seconds: lo.value(),
                }
            })
        });
        match result {
            Ok(encoded) => {
                // SAFETY: `out` was checked for null and the safety contract
                // requires it to point to writable `TempochTime` storage.
                unsafe { *out = encoded };
                TempochStatus::Ok
            }
            Err(err) => status_from_conversion(err),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpst_epoch_instant() -> TempochTime {
        // GPST week-0/second-0 epoch in J2000 seconds.
        TempochTime {
            hi_seconds: -630_763_200.0,
            lo_seconds: 0.0,
        }
    }

    #[test]
    fn gpst_epoch_round_trips_to_week_zero() {
        let mut gw = TempochGnssWeek {
            week: u32::MAX,
            seconds_of_week: u32::MAX,
            subsecond_nanos: u32::MAX,
        };
        let status = unsafe {
            tempoch_time_to_gnss_week(gpst_epoch_instant(), TempochScaleTag::GPST as i32, &mut gw)
        };
        assert_eq!(status, TempochStatus::Ok);
        assert_eq!(gw.week, 0);
        assert_eq!(gw.seconds_of_week, 0);
        assert_eq!(gw.subsecond_nanos, 0);

        let mut back = TempochTime {
            hi_seconds: 0.0,
            lo_seconds: 0.0,
        };
        let status =
            unsafe { tempoch_time_from_gnss_week(gw, TempochScaleTag::GPST as i32, &mut back) };
        assert_eq!(status, TempochStatus::Ok);
        assert!((back.hi_seconds + back.lo_seconds - (-630_763_200.0)).abs() < 1e-6);
    }

    #[test]
    fn non_gnss_scale_is_rejected() {
        let mut gw = TempochGnssWeek {
            week: 0,
            seconds_of_week: 0,
            subsecond_nanos: 0,
        };
        let status = unsafe {
            tempoch_time_to_gnss_week(gpst_epoch_instant(), TempochScaleTag::TT as i32, &mut gw)
        };
        assert_eq!(status, TempochStatus::InvalidScaleId);
    }

    #[test]
    fn null_pointer_is_rejected() {
        let status = unsafe {
            tempoch_time_to_gnss_week(
                gpst_epoch_instant(),
                TempochScaleTag::GPST as i32,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, TempochStatus::NullPointer);
    }
}
