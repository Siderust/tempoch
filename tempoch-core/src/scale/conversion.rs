// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scale conversion matrix.

use crate::constats::{IAU_TIME_EPOCH_T0_JD, L_B, L_G, TDB0, TT_MINUS_TAI};
use crate::context::TimeContext;
use crate::data::active::{active_time_data, time_data_delta_t, time_data_try_tai_minus_utc_mjd};
use crate::delta_t::delta_t_seconds;
use crate::encoding::{
    j2000_seconds_to_jd, jd_to_j2000_seconds, jd_to_julian_centuries, jd_to_mjd,
};
use crate::error::ConversionError;
use crate::scale::{Scale, TAI, TCB, TCG, TDB, TT, UT1, UTC};
use crate::sealed::Sealed;
use affn::algebra::{AffineMap1, Space, SplitPoint1, SplitQuantity};
use qtty::unit::{Day, Second as SecondUnit};
use qtty::{Day as JdDay, Second};

#[derive(Debug, Copy, Clone)]
struct SourceAxis;
impl Space for SourceAxis {}

#[derive(Debug, Copy, Clone)]
struct TargetAxis;
impl Space for TargetAxis {}

#[inline]
fn normalize_pair(hi: f64, lo: f64) -> (Second, Second) {
    SplitQuantity::<SecondUnit>::new(Second::new(hi), Second::new(lo)).pair()
}

#[inline]
fn add_constant(src_hi: Second, src_lo: Second, offset: Second) -> (Second, Second) {
    SplitQuantity::<SecondUnit>::new(src_hi, src_lo)
        .add_quantity(offset)
        .pair()
}

#[inline]
fn total_seconds(src_hi: Second, src_lo: Second) -> Second {
    SplitQuantity::<SecondUnit>::new(src_hi, src_lo).total()
}

#[inline]
fn linear_map_pair(
    src_hi: Second,
    src_lo: Second,
    source_origin: Second,
    target_origin: Second,
    scale: f64,
) -> (Second, Second) {
    let map =
        AffineMap1::<SourceAxis, TargetAxis, SecondUnit>::new(source_origin, target_origin, scale);
    map.apply_split_point(SplitPoint1::<SourceAxis, SecondUnit>::new(src_hi, src_lo))
        .coordinate()
        .pair()
}

#[inline]
fn tdb_minus_tt_seconds(jd_tt: JdDay) -> Second {
    // Source: USNO Circular 179 truncated seven-term Fairhead-Bretagnon
    // approximation for TDB - TT. The documented high-accuracy regime for this
    // specific truncation is about 10 microseconds over 1600-01-01 to
    // 2200-01-01 TT.
    let t = jd_to_julian_centuries(jd_tt);
    Second::new(
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
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD.raw());
        linear_map_pair(src_hi, src_lo, t0, t0, 1.0 / (1.0 - L_G))
    }
}

impl InfallibleScaleConvert<TT> for TCG {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD.raw());
        linear_map_pair(src_hi, src_lo, t0, t0, 1.0 - L_G)
    }
}

impl InfallibleScaleConvert<TCB> for TDB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        // TCB = T0 + (TDB - T0 - TDB0) / (1 - L_B)
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD.raw());
        linear_map_pair(
            src_hi,
            src_lo,
            t0,
            t0 - Second::new(TDB0.value() / (1.0 - L_B)),
            1.0 / (1.0 - L_B),
        )
    }
}

impl InfallibleScaleConvert<TDB> for TCB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        // TDB = T0 + (1 - L_B) * (TCB - T0) + TDB0
        let t0 = jd_to_j2000_seconds(IAU_TIME_EPOCH_T0_JD.raw());
        linear_map_pair(src_hi, src_lo, t0, t0 + TDB0, 1.0 - L_B)
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
                let (tai_hi, tai_lo) =
                    <UTC as InfallibleScaleConvert<TAI>>::convert(src_hi, src_lo);
                <TAI as InfallibleScaleConvert<$scale>>::convert(tai_hi, tai_lo)
            }
        }

        impl InfallibleScaleConvert<UTC> for $scale {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                let (tai_hi, tai_lo) =
                    <$scale as InfallibleScaleConvert<TAI>>::convert(src_hi, src_lo);
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
fn context_delta_t(jd_ut1: JdDay, ctx: &TimeContext) -> Result<Second, ConversionError> {
    let data = active_time_data();
    let mut mjd_utc = jd_to_mjd(jd_ut1);
    for _ in 0..2 {
        let Some(eop) = ctx.eop_at(mjd_utc) else {
            return time_data_delta_t(data.as_ref(), jd_ut1).or_else(|_| delta_t_seconds(jd_ut1));
        };
        mjd_utc = jd_to_mjd(jd_ut1 - eop.ut1_minus_utc.to::<Day>());
    }

    if let Some(eop) = ctx.eop_at(mjd_utc) {
        if let Ok(tai_minus_utc) = time_data_try_tai_minus_utc_mjd(data.as_ref(), mjd_utc, true) {
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
                let (tt_hi, tt_lo) =
                    <UT1 as ContextScaleConvert<TT>>::convert_with(src_hi, src_lo, ctx)?;
                Ok(<TT as InfallibleScaleConvert<$scale>>::convert(
                    tt_hi, tt_lo,
                ))
            }
        }

        impl ContextScaleConvert<UT1> for $scale {
            #[inline]
            fn convert_with(
                src_hi: Second,
                src_lo: Second,
                ctx: &TimeContext,
            ) -> Result<(Second, Second), ConversionError> {
                let (tt_hi, tt_lo) =
                    <$scale as InfallibleScaleConvert<TT>>::convert(src_hi, src_lo);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constats::TT_MINUS_TAI;
    use crate::delta_t::interpolate_modern_delta_t_points;
    use crate::generated::eop_data::{EOP_END_MJD, EOP_OBSERVED_END_MJD, EOP_POINTS};
    use crate::generated::time_data::{MODERN_DELTA_T_POINTS, UTC_TAI_SEGMENTS};
    use chrono::{Duration, NaiveDate};
    use qtty::Day as JulianDay;

    const TDB_TT_GOLDEN_SAMPLES: &[(f64, f64)] = &[
        // Curated from an independent maintenance-time reference script that
        // evaluated the published USNO Circular 179 seven-term series outside
        // the Rust conversion path. Columns: (JD_TT, TDB-TT seconds).
        (2_305_447.5, 0.000_132_413_656_208),
        (2_415_020.5, -0.000_018_411_200_301),
        (2_451_545.0, -0.000_095_757_434_861),
        (2_488_070.5, -0.000_056_504_654_873),
        (2_524_598.5, -0.000_053_339_853_687),
    ];

    fn tai_minus_utc_seconds_at_mjd(mjd_utc: f64) -> f64 {
        let idx = UTC_TAI_SEGMENTS.partition_point(|segment| segment.start_mjd as f64 <= mjd_utc);
        let segment = UTC_TAI_SEGMENTS[idx - 1];
        segment.base_seconds + segment.slope_seconds_per_day * (mjd_utc - segment.reference_mjd)
    }

    fn mjd_to_date_string(mjd: i32) -> String {
        let epoch = NaiveDate::from_ymd_opt(1858, 11, 17).expect("valid MJD epoch");
        (epoch + Duration::days(mjd as i64)).to_string()
    }

    #[test]
    fn tdb_minus_tt_matches_curated_circular_179_samples() {
        for &(jd_tt, expected_delta_seconds) in TDB_TT_GOLDEN_SAMPLES {
            let got = tdb_minus_tt_seconds(JulianDay::new(jd_tt)).value();
            let delta = (got - expected_delta_seconds).abs();
            assert!(
                delta < 1e-12,
                "TDB-TT mismatch at JD(TT) {jd_tt:.1}: got {got:.15e} s, expected {expected_delta_seconds:.15e} s, |Δ|={delta:.3e} s"
            );
        }
    }

    #[test]
    fn monthly_delta_t_stays_within_documented_daily_overlap_bounds() {
        let mut observed_worst: Option<(f64, i32, f64)> = None;
        let mut prediction_worst: Option<(f64, i32, f64)> = None;

        for point in EOP_POINTS.iter() {
            let mjd = point.mjd as f64;
            let Some(monthly_delta_t) =
                interpolate_modern_delta_t_points(&MODERN_DELTA_T_POINTS, JulianDay::new(mjd))
            else {
                continue;
            };
            let daily_delta_t = TT_MINUS_TAI.value() + tai_minus_utc_seconds_at_mjd(mjd)
                - point.ut1_minus_utc_seconds;
            let signed_diff = monthly_delta_t.value() - daily_delta_t;
            let candidate = (signed_diff.abs(), point.mjd, signed_diff);

            if point.ut1_observed {
                if observed_worst
                    .map(|worst| candidate.0 > worst.0)
                    .unwrap_or(true)
                {
                    observed_worst = Some(candidate);
                }
            } else if prediction_worst
                .map(|worst| candidate.0 > worst.0)
                .unwrap_or(true)
            {
                prediction_worst = Some(candidate);
            }
        }

        let (obs_abs, obs_mjd, obs_signed) = observed_worst.expect("observed EOP overlap");
        assert!(
            obs_abs < 0.015,
            "monthly ΔT exceeded observed-overlap bound: |Δ|={obs_abs:.9} s at MJD {obs_mjd} ({}) with signed diff {obs_signed:.9} s; expected < 0.015 s through {} ({})",
            mjd_to_date_string(obs_mjd),
            EOP_OBSERVED_END_MJD,
            mjd_to_date_string(EOP_OBSERVED_END_MJD),
        );

        let (pred_abs, pred_mjd, pred_signed) = prediction_worst.expect("prediction EOP overlap");
        assert!(
            pred_abs < 0.2,
            "monthly ΔT exceeded prediction-overlap bound: |Δ|={pred_abs:.9} s at MJD {pred_mjd} ({}) with signed diff {pred_signed:.9} s; expected < 0.2 s through {} ({})",
            mjd_to_date_string(pred_mjd),
            EOP_END_MJD,
            mjd_to_date_string(EOP_END_MJD),
        );
    }
}
