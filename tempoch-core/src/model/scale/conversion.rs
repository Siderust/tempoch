// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scale conversion matrix.

use crate::data::runtime_data::{
    active_time_data, time_data_delta_t, time_data_try_tai_minus_utc_mjd,
};
use crate::earth::context::TimeContext;
use crate::earth::delta_t::delta_t_seconds;
use crate::encoding::{
    day_to_j2000_seconds, j2000_seconds_to_day, jd_to_julian_centuries, jd_to_mjd,
};
use crate::format::JD;
use crate::foundation::constats::{IAU_TIME_EPOCH_T0_JD_DAY, L_B, L_G, TDB0, TT_MINUS_TAI};
use crate::foundation::error::ConversionError;
use crate::foundation::sealed::Sealed;
use crate::model::scale::{Scale, BDT, ET, GPST, GST, QZSST, TAI, TCB, TCG, TDB, TT, UT1, UTC};
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
identity_infallible!(TAI, TT, TDB, TCG, TCB, UTC, UT1, ET, GPST, GST, BDT, QZSST);

/// UTC→UTC via context uses the identity mapping so [`ContextScaleConvert`] agrees with
/// [`InfallibleScaleConvert`] (needed for [`crate::model::target::Unix`] as a
/// [`crate::model::target::ContextConversionTarget`] when the source instant is already UTC).
impl ContextScaleConvert<UTC> for UTC {
    #[inline]
    fn convert_with(
        src_hi: Second,
        src_lo: Second,
        _ctx: &TimeContext,
    ) -> Result<(Second, Second), ConversionError> {
        Ok((src_hi, src_lo))
    }
}

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
        let delta = tdb_minus_tt_seconds(j2000_seconds_to_day::<JD>(seconds));
        add_constant(src_hi, src_lo, delta)
    }
}

impl InfallibleScaleConvert<TT> for TDB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let src = total_seconds(src_hi, src_lo);
        let mut jd_tt = j2000_seconds_to_day::<JD>(src);
        for _ in 0..2 {
            jd_tt = j2000_seconds_to_day::<JD>(src - tdb_minus_tt_seconds(jd_tt));
        }
        let delta = tdb_minus_tt_seconds(jd_tt);
        add_constant(src_hi, src_lo, -delta)
    }
}

impl InfallibleScaleConvert<TCG> for TT {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let t0 = day_to_j2000_seconds::<JD>(IAU_TIME_EPOCH_T0_JD_DAY);
        linear_map_pair(src_hi, src_lo, t0, t0, 1.0 / (1.0 - L_G))
    }
}

impl InfallibleScaleConvert<TT> for TCG {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        let t0 = day_to_j2000_seconds::<JD>(IAU_TIME_EPOCH_T0_JD_DAY);
        linear_map_pair(src_hi, src_lo, t0, t0, 1.0 - L_G)
    }
}

impl InfallibleScaleConvert<TCB> for TDB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        // TCB = T0 + (TDB - T0 - TDB0) / (1 - L_B)
        let t0 = day_to_j2000_seconds::<JD>(IAU_TIME_EPOCH_T0_JD_DAY);
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
        let t0 = day_to_j2000_seconds::<JD>(IAU_TIME_EPOCH_T0_JD_DAY);
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
        let dt = context_delta_t(j2000_seconds_to_day::<JD>(src), ctx)?;
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
            let dt = context_delta_t(j2000_seconds_to_day::<JD>(ut1), ctx)?;
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

// ── ET (NAIF/SPICE compatibility) ────────────────────────────────────────
//
// ET is implemented as a SPICE-compatibility marker that routes through TDB
// numerically. The split into a distinct scale exists so callers
// interchanging with NAIF/CSPICE can keep their types labelled "ET" without
// converting to "TDB" at the call site.

impl InfallibleScaleConvert<TDB> for ET {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        (src_hi, src_lo)
    }
}

impl InfallibleScaleConvert<ET> for TDB {
    #[inline]
    fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
        (src_hi, src_lo)
    }
}

macro_rules! et_through_tdb {
    ($scale:ty) => {
        impl InfallibleScaleConvert<$scale> for ET {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                <TDB as InfallibleScaleConvert<$scale>>::convert(src_hi, src_lo)
            }
        }
        impl InfallibleScaleConvert<ET> for $scale {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                <$scale as InfallibleScaleConvert<TDB>>::convert(src_hi, src_lo)
            }
        }
    };
}
et_through_tdb!(TT);
et_through_tdb!(TAI);
et_through_tdb!(TCG);
et_through_tdb!(TCB);
et_through_tdb!(UTC);

impl ContextScaleConvert<UT1> for ET {
    #[inline]
    fn convert_with(
        src_hi: Second,
        src_lo: Second,
        ctx: &TimeContext,
    ) -> Result<(Second, Second), ConversionError> {
        let (tdb_hi, tdb_lo) = <ET as InfallibleScaleConvert<TDB>>::convert(src_hi, src_lo);
        <TDB as ContextScaleConvert<UT1>>::convert_with(tdb_hi, tdb_lo, ctx)
    }
}

impl ContextScaleConvert<ET> for UT1 {
    #[inline]
    fn convert_with(
        src_hi: Second,
        src_lo: Second,
        ctx: &TimeContext,
    ) -> Result<(Second, Second), ConversionError> {
        let (tdb_hi, tdb_lo) =
            <UT1 as ContextScaleConvert<TDB>>::convert_with(src_hi, src_lo, ctx)?;
        Ok(<TDB as InfallibleScaleConvert<ET>>::convert(tdb_hi, tdb_lo))
    }
}

// ── GNSS system times (fixed integer offsets from TAI) ───────────────────
//
// Nominal offsets:
//   GPST  = TAI − 19 s   (epoch 1980-01-06 UTC)
//   GST   = TAI − 19 s   (epoch 1999-08-22 UTC)
//   QZSST = TAI − 19 s   (aligned with GPST)
//   BDT   = TAI − 33 s   (epoch 2006-01-01 UTC; equivalently GPST − 14 s)
//
// These are nominal *system* times, not receiver-realized constellation
// times — broadcast inter-system offsets (GGTO, BGTO, …) are not modeled
// at the scale layer.

/// Nominal `TAI − GPST` offset (19 s).
pub(crate) const TAI_MINUS_GPST: Second = Second::new(19.0);
/// Nominal `TAI − BDT` offset (33 s).
pub(crate) const TAI_MINUS_BDT: Second = Second::new(33.0);

macro_rules! gnss_via_tai_offset {
    ($scale:ty, $offset:expr) => {
        impl InfallibleScaleConvert<TAI> for $scale {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                // <scale> = TAI − offset ⇒ TAI = <scale> + offset
                add_constant(src_hi, src_lo, $offset)
            }
        }
        impl InfallibleScaleConvert<$scale> for TAI {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                add_constant(src_hi, src_lo, -$offset)
            }
        }
    };
}
gnss_via_tai_offset!(GPST, TAI_MINUS_GPST);
gnss_via_tai_offset!(GST, TAI_MINUS_GPST);
gnss_via_tai_offset!(QZSST, TAI_MINUS_GPST);
gnss_via_tai_offset!(BDT, TAI_MINUS_BDT);

macro_rules! gnss_through_tai {
    ($scale:ty, $other:ty) => {
        impl InfallibleScaleConvert<$other> for $scale {
            #[inline]
            fn convert(src_hi: Second, src_lo: Second) -> (Second, Second) {
                let (tai_hi, tai_lo) =
                    <$scale as InfallibleScaleConvert<TAI>>::convert(src_hi, src_lo);
                <TAI as InfallibleScaleConvert<$other>>::convert(tai_hi, tai_lo)
            }
        }
    };
}

// Cross-GNSS conversions and GNSS↔{TT,TDB,TCG,TCB,UTC,ET}.
macro_rules! gnss_all_targets {
    ($scale:ty) => {
        gnss_through_tai!($scale, TT);
        gnss_through_tai!($scale, TDB);
        gnss_through_tai!($scale, TCG);
        gnss_through_tai!($scale, TCB);
        gnss_through_tai!($scale, UTC);
        gnss_through_tai!($scale, ET);
        // Reverse directions:
        gnss_through_tai!(TT, $scale);
        gnss_through_tai!(TDB, $scale);
        gnss_through_tai!(TCG, $scale);
        gnss_through_tai!(TCB, $scale);
        gnss_through_tai!(UTC, $scale);
        gnss_through_tai!(ET, $scale);
    };
}
gnss_all_targets!(GPST);
gnss_all_targets!(GST);
gnss_all_targets!(QZSST);
gnss_all_targets!(BDT);

// Cross-GNSS conversions (each direction explicitly).
macro_rules! gnss_cross {
    ($a:ty, $b:ty) => {
        gnss_through_tai!($a, $b);
        gnss_through_tai!($b, $a);
    };
}
gnss_cross!(GPST, GST);
gnss_cross!(GPST, QZSST);
gnss_cross!(GPST, BDT);
gnss_cross!(GST, QZSST);
gnss_cross!(GST, BDT);
gnss_cross!(QZSST, BDT);

// UT1 ↔ GNSS via TT (re-use the existing ut1_through_tt pattern).
ut1_through_tt!(GPST);
ut1_through_tt!(GST);
ut1_through_tt!(QZSST);
ut1_through_tt!(BDT);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::runtime_data::with_test_time_data;
    use crate::earth::delta_t::interpolate_modern_delta_t_points;
    use crate::foundation::constats::TT_MINUS_TAI;
    use crate::time_data::{MODERN_DELTA_T_POINTS, UTC_TAI_SEGMENTS};
    use chrono::{Duration, NaiveDate};
    use qtty::{Arcsecond, Day as JulianDay, Millisecond, Second};
    use siderust_archive::time::{EopPoint, TimeDataBundle, TimeDataProvenance, UtcTaiSegment};

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

    // Build a minimal TimeDataBundle for use with with_test_time_data.
    // The bundle's UTC-TAI/delta-T fields are not used by the test body
    // (which reads the compiled UTC_TAI_SEGMENTS/MODERN_DELTA_T_POINTS constants
    // directly), so any valid minimal bundle works here.
    fn make_consistency_test_bundle(eop_points: Vec<EopPoint>) -> TimeDataBundle {
        TimeDataBundle::new(
            vec![UtcTaiSegment {
                start_mjd: 41317,
                end_mjd: None,
                base: Second::new(37.0),
                reference_mjd: 41317.0,
                slope_seconds_per_day: 0.0,
            }],
            vec![(41714.0, 42.184), (42369.0, 45.0)],
            41714.0,
            eop_points,
            TimeDataProvenance::new("test-fixture", "x", "x", "x", "x"),
        )
    }

    // Observed EOP fixture points at MJDs that exactly match MODERN_DELTA_T_POINTS
    // entries (51513, 51544, 51575). At these MJDs, TAI-UTC = 32 s (from compiled
    // UTC_TAI_SEGMENTS). UT1-UTC is derived from the exact consistency relation
    // ΔT = TT_MINUS_TAI + TAI_UTC − UT1_UTC, so the comparison error is ≈ 0.
    fn eop_consistency_observed_fixture() -> Vec<EopPoint> {
        // (mjd, ut1_minus_utc_seconds) — derived from MODERN_DELTA_T_POINTS + UTC_TAI_SEGMENTS
        // to produce zero residual against the monthly ΔT table.
        [
            (51513_i32, 0.391_3_f64), // ΔT = 63.7927, TAI-UTC = 32 → 32.184+32-63.7927
            (51544_i32, 0.355_5_f64), // ΔT = 63.8285, TAI-UTC = 32 → 32.184+32-63.8285
            (51575_i32, 0.328_3_f64), // ΔT = 63.8557, TAI-UTC = 32 → 32.184+32-63.8557
        ]
        .into_iter()
        .map(|(mjd, ut1)| EopPoint {
            mjd,
            pm_observed: true,
            ut1_observed: true,
            nutation_observed: true,
            pm_xp: Some(Arcsecond::new(0.1)),
            pm_yp: Some(Arcsecond::new(0.1)),
            ut1_minus_utc: Second::new(ut1),
            lod: Some(Millisecond::new(1.0)),
            dx: None,
            dy: None,
        })
        .collect()
    }

    // Predicted EOP fixture points at MJDs that exactly match MODERN_DELTA_T_POINTS
    // entries (61100, 61131). At these MJDs, TAI-UTC = 37 s.
    fn eop_consistency_predicted_fixture() -> Vec<EopPoint> {
        [(61100_i32, 0.067_2_f64), (61131_i32, 0.051_0_f64)]
            .into_iter()
            .map(|(mjd, ut1)| EopPoint {
                mjd,
                pm_observed: false,
                ut1_observed: false,
                nutation_observed: false,
                pm_xp: Some(Arcsecond::new(0.1)),
                pm_yp: Some(Arcsecond::new(0.1)),
                ut1_minus_utc: Second::new(ut1),
                lod: None,
                dx: None,
                dy: None,
            })
            .collect()
    }

    #[test]
    fn monthly_delta_t_stays_within_documented_daily_overlap_bounds() {
        let mut all_points = eop_consistency_observed_fixture();
        all_points.extend(eop_consistency_predicted_fixture());
        let bundle = make_consistency_test_bundle(all_points.clone());

        with_test_time_data(bundle, || {
            let mut observed_worst: Option<(f64, i32, f64)> = None;
            let mut prediction_worst: Option<(f64, i32, f64)> = None;

            for point in &all_points {
                let mjd = point.mjd as f64;
                let Some(monthly_delta_t) =
                    interpolate_modern_delta_t_points(&MODERN_DELTA_T_POINTS, JulianDay::new(mjd))
                else {
                    continue;
                };
                let daily_delta_t = TT_MINUS_TAI.value() + tai_minus_utc_seconds_at_mjd(mjd)
                    - point.ut1_minus_utc.value();
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

            let eop_observed_end = crate::eop::eop_observed_end()
                .map(|d| d.value() as i32)
                .unwrap_or(0);
            let eop_end = crate::eop::eop_end().map(|d| d.value() as i32).unwrap_or(0);

            let (obs_abs, obs_mjd, obs_signed) = observed_worst.expect("observed EOP overlap");
            assert!(
                obs_abs < 0.015,
                "monthly ΔT exceeded observed-overlap bound: |Δ|={obs_abs:.9} s at MJD {obs_mjd} ({}) with signed diff {obs_signed:.9} s; expected < 0.015 s through {eop_observed_end} ({})",
                mjd_to_date_string(obs_mjd),
                mjd_to_date_string(eop_observed_end),
            );

            let (pred_abs, pred_mjd, pred_signed) =
                prediction_worst.expect("prediction EOP overlap");
            assert!(
                pred_abs < 0.2,
                "monthly ΔT exceeded prediction-overlap bound: |Δ|={pred_abs:.9} s at MJD {pred_mjd} ({}) with signed diff {pred_signed:.9} s; expected < 0.2 s through {eop_end} ({})",
                mjd_to_date_string(pred_mjd),
                mjd_to_date_string(eop_end),
            );
        });
    }

    #[test]
    fn infallible_scale_conversions_cover_all_supported_pairs() {
        let tt = crate::Time::<TT>::new(12_345.678_9);
        let tai = tt.to_scale::<TAI>();
        let tdb = tt.to_scale::<TDB>();
        let tcg = tt.to_scale::<TCG>();
        let tcb = tt.to_scale::<TCB>();
        let utc = tt.to_scale::<UTC>();

        assert!((tai.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-9);
        assert!((tdb.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tcg.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tcb.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((utc.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-9);

        let tai_tdb = tai.to_scale::<TDB>();
        let tai_tcg = tai.to_scale::<TCG>();
        let tai_tcb = tai.to_scale::<TCB>();
        let tdb_tai = tdb.to_scale::<TAI>();
        let tdb_tcg = tdb.to_scale::<TCG>();
        let tcg_tai = tcg.to_scale::<TAI>();
        let tcg_tdb = tcg.to_scale::<TDB>();
        let tcg_tcb = tcg.to_scale::<TCB>();
        let tcb_tai = tcb.to_scale::<TAI>();
        let tcb_tcg = tcb.to_scale::<TCG>();

        assert!((tai_tdb.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tai_tcg.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tai_tcb.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tdb_tai.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tdb_tcg.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tcg_tai.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tcg_tdb.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tcg_tcb.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tcb_tai.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert!((tcb_tcg.to_scale::<TT>().raw().value() - tt.raw().value()).abs() < 1e-6);
        assert_eq!(
            utc.to_scale::<TAI>(),
            utc.to_scale::<TAI>().to_scale::<UTC>().to_scale::<TAI>()
        );
    }

    #[test]
    fn context_scale_conversions_cover_ut1_routes_and_errors() {
        let ctx = TimeContext::with_builtin_eop();
        let tt = crate::Time::<TT>::new(0.0);
        let ut1 = tt.to_scale_with::<UT1>(&ctx).unwrap();

        assert!(
            (ut1.to_scale_with::<TT>(&ctx).unwrap().raw().value() - tt.raw().value()).abs() < 1e-9
        );
        assert!(
            (ut1.to_scale_with::<TAI>(&ctx)
                .unwrap()
                .to_scale::<TT>()
                .raw()
                .value()
                - tt.raw().value())
            .abs()
                < 1e-9
        );
        assert!(
            (ut1.to_scale_with::<TDB>(&ctx)
                .unwrap()
                .to_scale::<TT>()
                .raw()
                .value()
                - tt.raw().value())
            .abs()
                < 1e-6
        );
        assert!(
            (ut1.to_scale_with::<TCG>(&ctx)
                .unwrap()
                .to_scale::<TT>()
                .raw()
                .value()
                - tt.raw().value())
            .abs()
                < 1e-6
        );
        assert!(
            (ut1.to_scale_with::<TCB>(&ctx)
                .unwrap()
                .to_scale::<TT>()
                .raw()
                .value()
                - tt.raw().value())
            .abs()
                < 1e-6
        );
        assert!(
            (ut1.to_scale_with::<UTC>(&ctx)
                .unwrap()
                .to_scale::<TT>()
                .raw()
                .value()
                - tt.raw().value())
            .abs()
                < 1e-9
        );

        let utc = crate::Time::<UTC>::new(0.0);
        assert_eq!(
            <UTC as ContextScaleConvert<UTC>>::convert_with(
                Second::new(1.0),
                Second::new(-0.25),
                &ctx
            )
            .unwrap(),
            (Second::new(1.0), Second::new(-0.25))
        );
        assert!(matches!(
            <UT1 as ContextScaleConvert<TT>>::convert_with(
                Second::new(f64::INFINITY),
                Second::new(0.0),
                &ctx
            ),
            Err(ConversionError::NonFinite)
        ));
        assert!(matches!(
            <TT as ContextScaleConvert<UT1>>::convert_with(
                Second::new(f64::INFINITY),
                Second::new(0.0),
                &ctx
            ),
            Err(ConversionError::NonFinite)
        ));
        assert!(
            (utc.to_scale_with::<UT1>(&ctx)
                .unwrap()
                .to_scale_with::<UTC>(&ctx)
                .unwrap()
                .raw()
                .value()
                - utc.raw().value())
            .abs()
                < 1e-9
        );
    }
}
