// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

mod eop;
mod store;
mod utc_tai;

pub(crate) use eop::{time_data_delta_t, time_data_eop_at};
pub(crate) use store::active_time_data;
#[cfg(test)]
pub(crate) use store::{
    compiled_time_data, select_time_data, select_time_data_for_auto_refresh, set_active_time_data,
    with_runtime_data_lock, with_test_time_data,
};
#[cfg(feature = "runtime-data-fetch")]
pub use store::{fetch_latest_time_data, refresh_runtime_time_data, update_runtime_time_data};
pub(crate) use utc_tai::{
    time_data_tai_seconds_from_utc, time_data_tai_seconds_is_in_leap_window,
    time_data_try_tai_minus_utc_mjd, time_data_utc_from_tai_seconds,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{JulianDate, Unix, JD};
    use crate::{Time, TimeContext, TT, UT1, UTC};
    use chrono::DateTime;
    use qtty::Day as DayQuantity;
    use qtty::Second;
    use tempoch_time_data::TimeDataBundle;
    #[cfg(any(test, feature = "runtime-data-fetch"))]
    use tempoch_time_data::TimeDataError as InternalDataError;
    use tempoch_time_data::TimeDataProvenance;

    fn compiled_bundle_owned() -> TimeDataBundle {
        (*compiled_time_data()).clone()
    }

    fn bundle_with_timestamp(timestamp: &str) -> TimeDataBundle {
        let bundle = compiled_bundle_owned();
        TimeDataBundle::new(
            bundle.utc_tai_segments().to_vec(),
            bundle.modern_delta_t_points().to_vec(),
            bundle.modern_delta_t_observed_end_mjd(),
            bundle.eop_points().to_vec(),
            TimeDataProvenance::new(timestamp, "a", "b", "c", "d"),
        )
    }

    #[test]
    fn cache_is_selected_when_not_forcing_refresh() {
        let cached = bundle_with_timestamp("cached");
        let selected = select_time_data(
            Ok(cached.clone()),
            || {
                Err(InternalDataError::Integrity(
                    "refresh should not be called".into(),
                ))
            },
            false,
        )
        .unwrap();
        assert_eq!(selected.provenance().fetched_utc(), "cached");
    }

    #[test]
    fn missing_cache_triggers_refresh() {
        let refreshed = bundle_with_timestamp("refreshed");
        let selected = select_time_data(
            Err(InternalDataError::Integrity("missing cache".into())),
            || Ok(refreshed.clone()),
            false,
        )
        .unwrap();
        assert_eq!(selected.provenance().fetched_utc(), "refreshed");
    }

    #[test]
    fn force_refresh_ignores_cache() {
        let cached = bundle_with_timestamp("cached");
        let refreshed = bundle_with_timestamp("refreshed");
        let selected = select_time_data(Ok(cached), || Ok(refreshed.clone()), true).unwrap();
        assert_eq!(selected.provenance().fetched_utc(), "refreshed");
    }

    #[test]
    fn force_refresh_propagates_refresh_error() {
        let err = select_time_data(
            Ok(bundle_with_timestamp("cached")),
            || Err(InternalDataError::Download("network unreachable".into())),
            true,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("network unreachable"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn stale_cache_prefers_refresh_but_falls_back_if_refresh_fails() {
        let stale = bundle_with_timestamp("2026-04-15T00:00:00");
        let now = DateTime::from_timestamp(1_776_134_400, 0).unwrap();
        let selected = select_time_data_for_auto_refresh(
            Ok(stale.clone()),
            || Err(InternalDataError::Download("network unreachable".into())),
            now,
        )
        .unwrap();
        assert_eq!(
            selected.provenance().fetched_utc(),
            stale.provenance().fetched_utc()
        );
    }

    #[test]
    fn fresh_cache_skips_refresh_in_auto_mode() {
        let fresh = bundle_with_timestamp("2026-04-20T00:00:00");
        let now = DateTime::from_timestamp(1_776_139_200, 0).unwrap();
        let selected = select_time_data_for_auto_refresh(
            Ok(fresh.clone()),
            || {
                Err(InternalDataError::Integrity(
                    "refresh should not be called".into(),
                ))
            },
            now,
        )
        .unwrap();
        assert_eq!(
            selected.provenance().fetched_utc(),
            fresh.provenance().fetched_utc()
        );
    }

    #[test]
    fn ordinary_ut1_api_uses_override_bundle() {
        let bundle = compiled_bundle_owned();
        let mut eop_points = bundle.eop_points().to_vec();
        let point = eop_points.iter().position(|p| p.mjd == 57_000).unwrap();
        eop_points[point].ut1_minus_utc_seconds += 0.5;
        let bundle = TimeDataBundle::new(
            bundle.utc_tai_segments().to_vec(),
            bundle.modern_delta_t_points().to_vec(),
            bundle.modern_delta_t_observed_end_mjd(),
            eop_points,
            bundle.provenance().clone(),
        );

        with_test_time_data(bundle, || {
            let ctx = TimeContext::with_builtin_eop();
            let tt = Time::<TT>::from_raw_j2000_seconds(crate::encoding::day_to_j2000_seconds::<
                crate::format::JD,
            >(DayQuantity::new(
                2_400_000.5 + 57_000.0,
            )))
            .unwrap();
            let compiled = {
                let data = compiled_time_data();
                time_data_eop_at(data.as_ref(), DayQuantity::new(57_000.0))
                    .unwrap()
                    .ut1_minus_utc
            };
            let overridden = ctx.ut1_minus_utc(DayQuantity::new(57_000.0)).unwrap();
            assert!((overridden - compiled).abs() > Second::new(0.1));

            let ut1: Time<UT1> = tt.to_scale_with::<UT1>(&ctx).unwrap();
            assert!(ut1.to::<JD>().raw().is_finite());
        });
    }

    #[test]
    fn time_context_snapshots_ut1_data_across_active_bundle_updates() {
        with_runtime_data_lock(|| {
            let baseline = compiled_bundle_owned();
            let previous = active_time_data();
            set_active_time_data(baseline.clone());
            let ctx_before = TimeContext::with_builtin_eop();

            let mut eop_points = baseline.eop_points().to_vec();
            let point = eop_points.iter().position(|p| p.mjd == 57_000).unwrap();
            eop_points[point].ut1_minus_utc_seconds += 0.5;
            let overridden = TimeDataBundle::new(
                baseline.utc_tai_segments().to_vec(),
                baseline.modern_delta_t_points().to_vec(),
                baseline.modern_delta_t_observed_end_mjd(),
                eop_points,
                baseline.provenance().clone(),
            );
            set_active_time_data(overridden);
            let ctx_after = TimeContext::with_builtin_eop();

            let before = ctx_before
                .ut1_minus_utc(DayQuantity::new(57_000.0))
                .unwrap();
            let after = ctx_after.ut1_minus_utc(DayQuantity::new(57_000.0)).unwrap();
            set_active_time_data((*previous).clone());

            assert!((after - before).abs() > Second::new(0.1));
        });
    }

    #[test]
    fn ordinary_utc_api_uses_override_bundle() {
        let bundle = compiled_bundle_owned();
        let mut segments = bundle.utc_tai_segments().to_vec();
        let segment = segments
            .iter()
            .position(|segment| segment.start_mjd <= 60_000 && segment.end_mjd.is_none())
            .unwrap();
        segments[segment].base_seconds += 1.0;
        let bundle = TimeDataBundle::new(
            segments,
            bundle.modern_delta_t_points().to_vec(),
            bundle.modern_delta_t_observed_end_mjd(),
            bundle.eop_points().to_vec(),
            bundle.provenance().clone(),
        );
        let unix = Second::new(1_680_000_000.25);
        let compiled_value = {
            let compiled = compiled_time_data();
            let jd_utc = crate::encoding::unix_seconds_to_jd(unix);
            let mjd_utc = crate::encoding::jd_to_mjd(jd_utc);
            let tai_minus_utc =
                time_data_try_tai_minus_utc_mjd(compiled.as_ref(), mjd_utc, false).unwrap();
            (crate::encoding::day_to_j2000_seconds::<JD>(jd_utc) + tai_minus_utc).value()
        };

        with_test_time_data(bundle, || {
            let overridden = Time::<UTC, Unix>::try_new_with(unix, &TimeContext::new()).unwrap();
            let overridden_value =
                overridden.raw_seconds_pair().0.value() + overridden.raw_seconds_pair().1.value();
            assert!((overridden_value - compiled_value).abs() > 0.1);
            let roundtrip = overridden
                .raw_unix_seconds_with(&TimeContext::new())
                .unwrap();
            assert!((roundtrip - unix).abs() < Second::new(1e-3));
            let chrono = overridden.try_to_chrono().unwrap();
            let from_chrono = Time::<UTC>::try_from_chrono(chrono).unwrap();
            let drift = ((from_chrono.raw_seconds_pair().0.value()
                + from_chrono.raw_seconds_pair().1.value())
                - overridden_value)
                .abs();
            assert!(drift < 1e-4, "chrono round-trip drift = {drift}");
        });
    }

    #[test]
    fn time_context_snapshots_utc_civil_data_across_active_bundle_updates() {
        with_runtime_data_lock(|| {
            let baseline = compiled_bundle_owned();
            let previous = active_time_data();
            set_active_time_data(baseline.clone());
            let ctx_before = TimeContext::new();

            let mut segments = baseline.utc_tai_segments().to_vec();
            let segment = segments
                .iter()
                .position(|segment| segment.start_mjd <= 60_000 && segment.end_mjd.is_none())
                .unwrap();
            segments[segment].base_seconds += 1.0;
            let overridden = TimeDataBundle::new(
                segments,
                baseline.modern_delta_t_points().to_vec(),
                baseline.modern_delta_t_observed_end_mjd(),
                baseline.eop_points().to_vec(),
                baseline.provenance().clone(),
            );
            set_active_time_data(overridden);
            let ctx_after = TimeContext::new();

            let unix = Second::new(1_680_000_000.25);
            let before = Time::<UTC, Unix>::try_new_with(unix, &ctx_before).unwrap();
            let after = Time::<UTC, Unix>::try_new_with(unix, &ctx_after).unwrap();
            let before_value =
                before.raw_seconds_pair().0.value() + before.raw_seconds_pair().1.value();
            let after_value =
                after.raw_seconds_pair().0.value() + after.raw_seconds_pair().1.value();
            set_active_time_data((*previous).clone());

            assert!((after_value - before_value).abs() > 0.1);
        });
    }

    #[test]
    fn pre_1961_utc_errors_by_default_and_roundtrips_with_opt_in() {
        let dt = DateTime::from_timestamp(-631_152_000, 250_000_000).unwrap();

        assert!(matches!(
            Time::<UTC>::try_from_chrono(dt),
            Err(crate::ConversionError::UtcBeforeDefinition)
        ));

        let ctx = TimeContext::new().allow_pre_definition_utc();
        let utc = Time::<UTC>::try_from_chrono_with(dt, &ctx).unwrap();
        let back = utc.try_to_chrono_with(&ctx).unwrap();
        let drift = (back.timestamp_nanos_opt().unwrap() - dt.timestamp_nanos_opt().unwrap()).abs();
        assert!(drift < 50_000, "pre-1961 UTC round-trip drift = {drift} ns");

        let unix = Second::new(-631_152_000.75);
        assert!(matches!(
            Time::<UTC, Unix>::try_new(unix),
            Err(crate::ConversionError::UtcBeforeDefinition)
        ));

        let utc_from_unix = Time::<UTC, Unix>::try_new_with(unix, &ctx).unwrap();
        let unix_back = utc_from_unix.raw_unix_seconds_with(&ctx).unwrap();
        assert!((unix_back - unix).abs() < Second::new(1e-3));
    }

    #[test]
    fn runtime_bundle_can_extend_delta_t_horizon_through_existing_api() {
        let bundle = compiled_bundle_owned();
        let mut points = bundle.modern_delta_t_points().to_vec();
        let last = *points.last().unwrap();
        points.push((last.0 + 31.0, last.1 + 0.25));
        let bundle = TimeDataBundle::new(
            bundle.utc_tai_segments().to_vec(),
            points,
            bundle.modern_delta_t_observed_end_mjd(),
            bundle.eop_points().to_vec(),
            bundle.provenance().clone(),
        );
        let beyond = crate::DELTA_T_PREDICTION_HORIZON_MJD + DayQuantity::new(15.0);
        let jd = beyond + crate::foundation::constats::JD_MINUS_MJD;
        let tt = JulianDate::<TT>::new(jd.value()).to_j2000s();

        assert_eq!(
            tt.to_scale_with::<UT1>(&TimeContext::new()).unwrap_err(),
            crate::ConversionError::Ut1HorizonExceeded
        );

        with_test_time_data(bundle, || {
            let ut1 = tt.to_scale_with::<UT1>(&TimeContext::new()).unwrap();
            assert!(ut1.to::<JD>().raw().is_finite());
        });
    }

    #[test]
    fn eop_lookup_returns_none_when_bundle_has_gap() {
        let bundle = compiled_bundle_owned();
        let mut eop_points = bundle.eop_points().to_vec();
        let gap_idx = eop_points
            .windows(2)
            .position(|window| window[1].mjd == window[0].mjd + 1)
            .expect("compiled EOP series should contain adjacent rows")
            + 1;
        let gap_after = eop_points[gap_idx - 1].mjd;
        eop_points.remove(gap_idx);
        let bundle = TimeDataBundle::new(
            bundle.utc_tai_segments().to_vec(),
            bundle.modern_delta_t_points().to_vec(),
            bundle.modern_delta_t_observed_end_mjd(),
            eop_points,
            bundle.provenance().clone(),
        );

        assert!(time_data_eop_at(&bundle, DayQuantity::new(gap_after as f64 + 0.5)).is_none());
        assert!(time_data_eop_at(&bundle, DayQuantity::new((gap_after + 1) as f64)).is_none());
    }

    #[test]
    fn compiled_bundle_is_available() {
        let bundle = compiled_time_data();
        assert!(!bundle.utc_tai_segments().is_empty());
        assert!(!bundle.modern_delta_t_points().is_empty());
        assert!(!bundle.eop_points().is_empty());
    }
}
