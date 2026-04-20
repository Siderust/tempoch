// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scale conversion matrix.

use crate::constats::{IAU_TIME_EPOCH_T0_JD, L_B, L_G, TDB0, TT_MINUS_TAI};
use crate::context::TimeContext;
use crate::data::active::{active_time_data, time_data_delta_t, time_data_try_tai_minus_utc_mjd};
use crate::delta_t::delta_t_seconds;
use crate::encoding::{j2000_seconds_to_jd, jd_to_j2000_seconds, jd_to_julian_centuries, jd_to_mjd};
use crate::error::ConversionError;
use crate::sealed::Sealed;
use crate::scale::{Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
use qtty::time::{Days, Seconds};
use qtty::unit::Day;
use qtty::Second;

#[inline]
fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    let err = (a - (s - bb)) + (b - bb);
    (s, err)
}

#[inline]
fn normalize_pair(hi: f64, lo: f64) -> (Second, Second) {
    let (sum, err) = two_sum(hi, lo);
    let (sum2, err2) = two_sum(sum, err);
    (Second::new(sum2), Second::new(err2))
}

#[inline]
fn add_constant(src_hi: Second, src_lo: Second, offset: Second) -> (Second, Second) {
    normalize_pair(src_hi.value(), src_lo.value() + offset.value())
}

#[inline]
fn total_seconds(src_hi: Second, src_lo: Second) -> Second {
    src_hi + src_lo
}

#[inline]
fn tdb_minus_tt_seconds(jd_tt: Days) -> Seconds {
    let t = jd_to_julian_centuries(jd_tt);
    Seconds::new(
        0.001_657 * (628.3076 * t + 6.2401).sin()
            + 0.000_022 * (575.3385 * t + 4.2970).sin()
            + 0.000_014 * (1256.6152 * t + 6.1969).sin()
            + 0.000_005 * (606.9777 * t + 4.0212).sin()
            + 0.000_005 * (52.9691 * t + 0.4444).sin()
            + 0.000_002 * (21.3299 * t + 5.5431).sin()
            + 0.000_010 * t * (628.3076 * t + 4.2490).sin(),
    )
}

pub(crate) trait InfallibleScaleConvert<S2: Scale>: Scale + Sealed {
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second);
}

pub(crate) trait ContextScaleConvert<S2: Scale>: Scale + Sealed {
    fn convert_with(
        src_hi: Second,
        src_lo: Second,
        ctx: &TimeContext,
    ) -> Result<(Second, Second), ConversionError>;
}

macro_rules! identity_infallible {
    ($($scale:ty),+ $(,)?) => {
        $(
            impl InfallibleScaleConvert<$scale> for $scale {
                #[inline]
                fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                    (src_hi, src_lo)
                }
            }
        )+
    };
}
identity_infallible!(TAI, TT, TDB, TCG, TCB, UTC, UT1);

impl InfallibleScaleConvert<TT> for TAI {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        add_constant(src_hi, src_lo, TT_MINUS_TAI)
    }
}

impl InfallibleScaleConvert<TAI> for TT {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        add_constant(src_hi, src_lo, -TT_MINUS_TAI)
    }
}

impl InfallibleScaleConvert<TDB> for TT {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let seconds = total_seconds(src_hi, src_lo);
        let delta = tdb_minus_tt_seconds(j2000_seconds_to_jd(seconds));
        add_constant(src_hi, src_lo, delta)
    }
}

impl InfallibleScaleConvert<TT> for TDB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let src = total_seconds(src_hi, src_lo);
        let mut jd_tt = j2000_seconds_to_jd(src);
        for _ in 0..2 {
            jd_tt = j2000_seconds_to_jd(src - tdb_minus_tt_seconds(jd_tt));
        }
        let delta = tdb_minus_tt_seconds(jd_tt);
        add_constant(src_hi, src_lo, -delta)
    }
}

impl InfallibleScaleConvert<TCG> for TT {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let src = total_seconds(src_hi, src_lo);
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = Second::new(L_G * (src - t0).value() / (1.0 - L_G));
        add_constant(src_hi, src_lo, delta)
    }
}

impl InfallibleScaleConvert<TT> for TCG {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let src = total_seconds(src_hi, src_lo);
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = Second::new(-L_G * (src - t0).value());
        add_constant(src_hi, src_lo, delta)
    }
}

impl InfallibleScaleConvert<TCB> for TDB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let src = total_seconds(src_hi, src_lo);
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = src - t0 - TDB0;
        let target = t0 + delta / (1.0 - L_B);
        normalize_pair(target.value(), 0.0)
    }
}

impl InfallibleScaleConvert<TDB> for TCB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let src = total_seconds(src_hi, src_lo);
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD);
        let delta = src - t0;
        let target = t0 + (1.0 - L_B) * delta + TDB0;
        normalize_pair(target.value(), 0.0)
    }
}

macro_rules! through_tt {
    ($from:ty, $to:ty) => {
        impl InfallibleScaleConvert<$to> for $from {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                let (tt_hi, tt_lo) = <$from as InfallibleScaleConvert<TT>>::convert(src_hi, src_lo);
                <TT as InfallibleScaleConvert<$to>>::convert(tt_hi, tt_lo)
            }
        }
    };
}

through_tt!(TAI, TDB);
through_tt!(TDB, TAI);
through_tt!(TAI, TCG);
through_tt!(TCG, TAI);
through_tt!(TAI, TCB);
through_tt!(TCB, TAI);
through_tt!(TDB, TCG);
through_tt!(TCG, TDB);
through_tt!(TCG, TCB);
through_tt!(TCB, TCG);

impl InfallibleScaleConvert<TCB> for TT {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let (tdb_hi, tdb_lo) = <TT as InfallibleScaleConvert<TDB>>::convert(src_hi, src_lo);
        <TDB as InfallibleScaleConvert<TCB>>::convert(tdb_hi, tdb_lo)
    }
}

impl InfallibleScaleConvert<TT> for TCB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let (tdb_hi, tdb_lo) = <TCB as InfallibleScaleConvert<TDB>>::convert(src_hi, src_lo);
        <TDB as InfallibleScaleConvert<TT>>::convert(tdb_hi, tdb_lo)
    }
}

impl InfallibleScaleConvert<TAI> for UTC {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        (src_hi, src_lo)
    }
}

impl InfallibleScaleConvert<UTC> for TAI {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        (src_hi, src_lo)
    }
}

macro_rules! utc_through_tai {
    ($scale:ty) => {
        impl InfallibleScaleConvert<$scale> for UTC {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                let (tai_hi, tai_lo) = <UTC as InfallibleScaleConvert<TAI>>::convert(src_hi, src_lo);
                <TAI as InfallibleScaleConvert<$scale>>::convert(tai_hi, tai_lo)
            }
        }

        impl InfallibleScaleConvert<UTC> for $scale {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                let (tai_hi, tai_lo) = <$scale as InfallibleScaleConvert<TAI>>::convert(src_hi, src_lo);
                <TAI as InfallibleScaleConvert<UTC>>::convert(tai_hi, tai_lo)
            }
        }
    };
}

utc_through_tai!(TT);
utc_through_tai!(TDB);
utc_through_tai!(TCG);
utc_through_tai!(TCB);

#[inline]
fn context_delta_t(jd_ut1: Days, ctx: &TimeContext) -> Result<Seconds, ConversionError> {
    let data = active_time_data();
    let mut mjd_utc = jd_to_mjd(jd_ut1);
    for _ in 0..2 {
        let Some(eop) = ctx.eop_at(mjd_utc) else {
            return time_data_delta_t(data.as_ref(), jd_ut1).or_else(|_| delta_t_seconds(jd_ut1));
        };
        mjd_utc = jd_to_mjd(jd_ut1 - eop.ut1_minus_utc.to::<Day>());
    }

    if let Some(eop) = ctx.eop_at(mjd_utc) {
        if let Some(tai_minus_utc) = time_data_try_tai_minus_utc_mjd(data.as_ref(), mjd_utc) {
            return Ok(TT_MINUS_TAI + tai_minus_utc - eop.ut1_minus_utc);
        }
    }

    time_data_delta_t(data.as_ref(), jd_ut1).or_else(|_| delta_t_seconds(jd_ut1))
}

impl ContextScaleConvert<TT> for UT1 {
    #[inline]
    fn convert_with(
        src_hi: Second,
        src_lo: Second,
        ctx: &TimeContext,
    ) -> Result<(Second, Second), ConversionError> {
        let src = total_seconds(src_hi, src_lo);
        if !src.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        let dt = context_delta_t(j2000_seconds_to_jd(src), ctx)?;
        Ok(add_constant(src_hi, src_lo, dt))
    }
}

impl ContextScaleConvert<UT1> for TT {
    #[inline]
    fn convert_with(
        src_hi: Second,
        src_lo: Second,
        ctx: &TimeContext,
    ) -> Result<(Second, Second), ConversionError> {
        let src = total_seconds(src_hi, src_lo);
        if !src.is_finite() {
            return Err(ConversionError::NonFinite);
        }
        let mut ut1 = src;
        for _ in 0..3 {
            let dt = context_delta_t(j2000_seconds_to_jd(ut1), ctx)?;
            ut1 = src - dt;
        }
        Ok(normalize_pair(ut1.value(), 0.0))
    }
}

macro_rules! ut1_through_tt {
    ($scale:ty) => {
        impl ContextScaleConvert<$scale> for UT1 {
            #[inline]
            fn convert_with(
                src_hi: Second,
                src_lo: Second,
                ctx: &TimeContext,
            ) -> Result<(Second, Second), ConversionError> {
                let (tt_hi, tt_lo) = <UT1 as ContextScaleConvert<TT>>::convert_with(src_hi, src_lo, ctx)?;
                Ok(<TT as InfallibleScaleConvert<$scale>>::convert(tt_hi, tt_lo))
            }
        }

        impl ContextScaleConvert<UT1> for $scale {
            #[inline]
            fn convert_with(
                src_hi: Second,
                src_lo: Second,
                ctx: &TimeContext,
            ) -> Result<(Second, Second), ConversionError> {
                let (tt_hi, tt_lo) = <$scale as InfallibleScaleConvert<TT>>::convert(src_hi, src_lo);
                <TT as ContextScaleConvert<UT1>>::convert_with(tt_hi, tt_lo, ctx)
            }
        }
    };
}

ut1_through_tt!(TAI);
ut1_through_tt!(TDB);
ut1_through_tt!(TCG);
ut1_through_tt!(TCB);
ut1_through_tt!(UTC);
