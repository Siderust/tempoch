// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use super::utc_tai::time_data_tai_minus_utc_mjd_extrapolated;
use crate::archive::time::{EopPoint, TimeDataBundle};
use crate::earth::delta_t::delta_t_seconds_from_modern_points;
use crate::earth::eop::EopValues;
use crate::foundation::error::ConversionError;
use qtty::Day as DayQuantity;
use qtty::Second;

pub(crate) fn time_data_delta_t(
    data: &TimeDataBundle,
    jd_ut: DayQuantity,
) -> Result<Second, ConversionError> {
    delta_t_seconds_from_modern_points(jd_ut, data.modern_delta_t_points())
}

pub(crate) fn time_data_eop_at(data: &TimeDataBundle, mjd_utc: DayQuantity) -> Option<EopValues> {
    let points = data.eop_points();
    let first = points.first()?.mjd;
    let last = points.last()?.mjd;
    let mjd_f = mjd_utc.value();
    let lo_i = mjd_f.floor() as i32;
    let hi_i = lo_i + 1;
    if lo_i < first || lo_i > last {
        return None;
    }
    let lo = find_eop_point(points, lo_i)?;
    let hi = if hi_i > last {
        lo
    } else {
        find_eop_point(points, hi_i)?
    };

    let frac = if lo.mjd == hi.mjd {
        0.0
    } else {
        mjd_f - lo_i as f64
    };
    let lerp = |a: f64, b: f64| a + frac * (b - a);
    let lerp_opt = |a: Option<f64>, b: Option<f64>| match (a, b) {
        (Some(a), Some(b)) => Some(lerp(a, b)),
        _ => None,
    };
    let lod_milliseconds = lerp_opt(lo.lod.map(|v| v.value()), hi.lod.map(|v| v.value()));

    let ut1_minus_utc = {
        // Allow extrapolation here: these calls are for internal ΔT bookkeeping
        // (correcting EOP-derived UT1-UTC to the actual UTC-TAI offset), not for
        // validating UTC representations. Pre-1961 EOP data is rare but valid.
        let lo_offset =
            time_data_tai_minus_utc_mjd_extrapolated(data, DayQuantity::new(lo_i as f64));
        let hi_offset =
            time_data_tai_minus_utc_mjd_extrapolated(data, DayQuantity::new(hi_i as f64));
        let query_offset = time_data_tai_minus_utc_mjd_extrapolated(data, mjd_utc);
        match (lo_offset, hi_offset, query_offset) {
            (Some(lo_tmu), Some(hi_tmu), Some(query_tmu)) => {
                let lo_cont = lo.ut1_minus_utc.value() - lo_tmu.value();
                let hi_cont = hi.ut1_minus_utc.value() - hi_tmu.value();
                Second::new(lerp(lo_cont, hi_cont) + query_tmu.value())
            }
            _ => Second::new(lerp(lo.ut1_minus_utc.value(), hi.ut1_minus_utc.value())),
        }
    };

    Some(EopValues {
        mjd_utc,
        pm_xp: lerp_opt(lo.pm_xp.map(|v| v.value()), hi.pm_xp.map(|v| v.value()))
            .map(qtty::f64::Arcsecond::new),
        pm_yp: lerp_opt(lo.pm_yp.map(|v| v.value()), hi.pm_yp.map(|v| v.value()))
            .map(qtty::f64::Arcsecond::new),
        ut1_minus_utc,
        lod: lod_milliseconds.map(qtty::f64::Millisecond::new),
        dx: lerp_opt(lo.dx.map(|v| v.value()), hi.dx.map(|v| v.value()))
            .map(qtty::f64::MilliArcsecond::new),
        dy: lerp_opt(lo.dy.map(|v| v.value()), hi.dy.map(|v| v.value()))
            .map(qtty::f64::MilliArcsecond::new),
        ut1_observed: lo.ut1_observed && hi.ut1_observed,
    })
}

fn find_eop_point(points: &[EopPoint], mjd: i32) -> Option<EopPoint> {
    let idx = points.partition_point(|point| point.mjd < mjd);
    let point = *points.get(idx)?;
    (point.mjd == mjd).then_some(point)
}
