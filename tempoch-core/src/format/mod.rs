// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Typed external formats for [`crate::Time`].
//!
//! A *format* marker specifies how a time instant is externally expressed.
//! The built-in markers live in [`markers`]: Julian Day (`JD`),
//! Modified Julian Day (`MJD`), J2000 seconds (`J2000s`), POSIX seconds
//! (`Unix`), and GPS seconds (`GPS`). Format is orthogonal to *scale*:
//! `JulianDate<TT>` and `JulianDate<UTC>` share the same format but live on
//! different physical time axes, and the compiler treats them as distinct,
//! incompatible types.
//!
//! Instants are always [`crate::Time<S, F>`] with compensated J2000-second
//! storage; `F` is a phantom encoding tag for `raw()`, conversions, and targets.
//!
//! [`JulianDate<S>`], [`ModifiedJulianDate<S>`], [`UnixTime`], and [`GpsTime`] implement
//! [`Into`] into the default-tagged [`crate::Time`] instant on their scale (`Time<S>`,
//! [`Time<UTC>`](crate::Time<crate::UTC>), [`Time<TAI>`](crate::Time<crate::TAI>)), equivalent to `Time::to_j2000s`.
//! [`crate::Interval::try_new`] therefore accepts encoded endpoints wherever `Into<crate::Time<S>>` is required (including [`crate::Period`]).
//!
//! [`J2000Seconds<S>`] is a type alias for [`crate::Time<S>`]; prefer it when you want an explicit name for the default tag.
//!
//! # Main types
//!
//! - [`TimeFormat`] — sealed marker trait (`JD`, `MJD`, …).
//! - [`FormatForScale<S>`] — witness that format `F` can encode scale `S`.
//! - [`InfallibleFormatForScale<S>`] — witness that the round-trip is
//!   context-free (except where the format itself requires a context, e.g. Unix).

mod time_format;
pub use time_format::TimeFormat;

pub mod markers;
pub use markers::{J2000s, Unix, GPS, JD, MJD};

mod traits;
pub use traits::{FormatForScale, InfallibleFormatForScale};

mod impls;

mod chrono;
pub mod iso;
pub use iso::{FormatOptions, FormatPrecision};
pub mod gnss_week;
pub use gnss_week::{GnssWeek, GnssWeekScale};

/// Julian day instant on scale `S` (`JD` tag).
pub type JulianDate<S> = crate::model::time::Time<S, JD>;
/// Modified Julian day instant on scale `S`.
pub type ModifiedJulianDate<S> = crate::model::time::Time<S, MJD>;
/// SI seconds since J2000.0 on scale `S`.
pub type J2000Seconds<S> = crate::model::time::Time<S, J2000s>;
/// POSIX / Unix seconds on the UTC axis.
pub type UnixTime = crate::model::time::Time<crate::model::scale::UTC, Unix>;
/// GPS seconds on the TAI axis.
pub type GpsTime = crate::model::time::Time<crate::model::scale::TAI, GPS>;

impl<S: crate::model::scale::Scale> From<JulianDate<S>> for crate::Time<S> {
    #[inline]
    fn from(value: JulianDate<S>) -> Self {
        value.to_j2000s()
    }
}

impl<S: crate::model::scale::Scale> From<ModifiedJulianDate<S>> for crate::Time<S> {
    #[inline]
    fn from(value: ModifiedJulianDate<S>) -> Self {
        value.to_j2000s()
    }
}

impl From<UnixTime> for crate::Time<crate::model::scale::UTC> {
    #[inline]
    fn from(value: UnixTime) -> Self {
        value.to_j2000s()
    }
}

impl From<GpsTime> for crate::Time<crate::model::scale::TAI> {
    #[inline]
    fn from(value: GpsTime) -> Self {
        value.to_j2000s()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::earth::context::TimeContext;
    use crate::model::scale::{TAI, TT, UTC};
    use crate::model::target::ConversionTarget;
    use crate::qtty::{Day, Second};

    #[test]
    fn encoded_time_display_delegates_to_quantity() {
        let jd = JulianDate::<TT>::new(2_451_545.123_456_789);

        assert_eq!(format!("{jd:.9}"), "2451545.123456789 d");
    }

    #[test]
    fn encoded_time_lower_exp_delegates_to_quantity() {
        let seconds = J2000Seconds::<TT>::new(1_234.5);
        let formatted = format!("{seconds:.2e}");

        assert_eq!(formatted, format!("{:.2e}", seconds.raw()));
    }

    #[test]
    fn encoded_time_upper_exp_delegates_to_quantity() {
        let seconds = J2000Seconds::<TT>::new(1_234.5);
        let formatted = format!("{seconds:.2E}");

        assert_eq!(formatted, format!("{:.2E}", seconds.raw()));
    }

    #[test]
    fn encoded_time_clone_matches_original() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let cloned = <JulianDate<TT> as Clone>::clone(&jd);
        assert_eq!(jd.raw(), cloned.raw());
    }

    #[test]
    fn encoded_time_partial_eq() {
        let a = JulianDate::<TT>::new(2_451_545.0);
        let b = JulianDate::<TT>::new(2_451_545.0);
        let c = JulianDate::<TT>::new(2_451_546.0);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn encoded_time_quantity_is_alias_for_raw() {
        let jd = JulianDate::<TT>::new(2_451_545.5);
        assert_eq!(jd.raw(), jd.quantity());
    }

    #[test]
    fn encoded_time_new_accepts_scalar_values() {
        let jd = JulianDate::<TT>::new(2_460_000.5);

        assert_eq!(jd.raw(), Day::new(2_460_000.5));
    }

    #[test]
    fn encoded_time_try_to_time_on_unix() {
        let ctx = TimeContext::new();
        let unix = UnixTime::try_new(Second::new(946_727_935.816)).unwrap();
        let time = unix.to_j2000s();
        let back = <Unix as crate::format::FormatForScale<UTC>>::try_from_time(time, &ctx).unwrap();
        assert!((back - Second::new(946_727_935.816)).abs() < Second::new(1e-3));
    }

    #[test]
    fn encoded_time_to_infallible_conversion() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let mjd: ModifiedJulianDate<TT> = jd.to::<MJD>();
        assert!((mjd.raw().value() - 51_544.5).abs() < 1e-9);
    }

    #[test]
    fn encoded_time_try_to_conversion() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let mjd: ModifiedJulianDate<TT> = jd.try_to::<MJD>().unwrap();
        assert!((mjd.raw().value() - 51_544.5).abs() < 1e-9);
    }

    #[test]
    fn encoded_time_to_with_for_unix() {
        let ctx = TimeContext::new();
        let jd = JulianDate::<UTC>::try_new_with(Day::new(2_451_545.0), &ctx).unwrap();
        let unix: UnixTime = jd.to_with::<Unix>(&ctx).unwrap();
        let unix_sec = unix.try_raw_with(&ctx).unwrap();
        assert!(unix_sec.value().is_finite());
        assert!(unix_sec.value() > 9e8 && unix_sec.value() < 1e10);
    }

    #[test]
    fn gps_format_roundtrip_through_tai() {
        let gps_seconds = Second::new(0.0);
        let time = <GPS as crate::format::InfallibleFormatForScale<TAI>>::into_time(gps_seconds);
        let back = <GPS as crate::format::InfallibleFormatForScale<TAI>>::from_time(time);
        assert!((back - gps_seconds).abs() < Second::new(1e-12));
    }

    #[test]
    fn gps_encoded_time_to_time_roundtrip() {
        let gps = GpsTime::new(1_234_567.89);
        let time = gps.to_j2000s();
        let back: GpsTime = time.to::<GPS>();
        assert!((back.raw() - gps.raw()).abs() < Second::new(1e-6));
    }

    #[test]
    fn from_encoded_time_into_time() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let time: crate::model::time::Time<TT> = jd.into();
        let back: JulianDate<TT> = time.to::<JD>();
        assert!((back.raw() - Day::new(2_451_545.0)).abs() < Day::new(1e-12));
    }

    #[test]
    fn encoded_into_default_time_matches_to_j2000s() {
        let jd = JulianDate::<TT>::new(2_451_545.25);
        let mjd = ModifiedJulianDate::<TT>::new(51_545.0);
        assert_eq!(crate::Time::<TT>::from(jd), jd.to_j2000s());
        assert_eq!(crate::Time::<TT>::from(mjd), mjd.to_j2000s());
        let unix = UnixTime::try_new(Second::new(1_700_000_000.0)).unwrap();
        assert_eq!(crate::Time::<UTC>::from(unix), unix.to_j2000s());
        let gps = GpsTime::new(100.0);
        assert_eq!(crate::Time::<TAI>::from(gps), gps.to_j2000s());
    }

    #[test]
    fn period_try_new_accepts_encoded_endpoints_via_into() {
        use crate::Period;

        let jd_a = JulianDate::<TT>::new(2_451_545.0);
        let jd_b = JulianDate::<TT>::new(2_451_546.0);
        let from_jd = Period::<TT>::try_new(jd_a, jd_b).unwrap();
        let explicit_jd = Period::<TT>::try_new(jd_a.to_j2000s(), jd_b.to_j2000s()).unwrap();
        assert_eq!(from_jd, explicit_jd);

        let mjd_a = ModifiedJulianDate::<TT>::new(51_544.0);
        let mjd_b = ModifiedJulianDate::<TT>::new(51_545.0);
        let from_mjd = Period::<TT>::try_new(mjd_a, mjd_b).unwrap();
        let explicit_mjd = Period::<TT>::try_new(mjd_a.to_j2000s(), mjd_b.to_j2000s()).unwrap();
        assert_eq!(from_mjd, explicit_mjd);
    }

    #[test]
    fn infallible_conversion_target_for_j2000s() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let time = jd.to_j2000s();
        let j2k: J2000Seconds<TT> = time.to::<J2000s>();
        assert!((j2k.raw().value()).abs() < 1e-6);
    }

    #[test]
    fn conversion_target_try_convert_for_j2000s() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let time = jd.to_j2000s();
        let j2k: J2000Seconds<TT> = time.try_to::<J2000s>().unwrap();
        assert!((j2k.raw().value()).abs() < 1e-6);
    }

    #[test]
    fn conversion_target_try_convert_for_jd() {
        let mjd = ModifiedJulianDate::<TT>::new(51_544.0);
        let time = mjd.to_j2000s();
        let jd: JulianDate<TT> = JD::try_convert(time).unwrap();
        assert!((jd.raw().value() - 2_451_544.5).abs() < 1e-9);
    }

    #[test]
    fn conversion_target_try_convert_for_mjd() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let time = jd.to_j2000s();
        let mjd: ModifiedJulianDate<TT> = MJD::try_convert(time).unwrap();
        assert!((mjd.raw().value() - 51_544.5).abs() < 1e-9);
    }

    #[test]
    fn gps_conversion_target_try_convert() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let time = jd.to_j2000s();
        let gps: GpsTime = GPS::try_convert(time).unwrap();
        assert!(gps.raw().is_finite());
    }

    #[test]
    fn unix_context_conversion_target() {
        let ctx = TimeContext::new();
        let jd = JulianDate::<UTC>::try_new_with(Day::new(2_451_545.0), &ctx).unwrap();
        let utc_time = jd.to_j2000s();
        let unix: crate::model::time::Time<UTC, Unix> =
            <Unix as crate::model::target::ContextConversionTarget<
                UTC,
                crate::format::J2000s,
            >>::convert_with(utc_time, &ctx)
            .unwrap();
        let unix_sec = unix.try_raw_with(&ctx).unwrap();
        assert!(unix_sec.value().is_finite());
        assert!(unix_sec.value() > 9e8 && unix_sec.value() < 1e10);
    }

    #[test]
    fn debug_includes_format_and_scale() {
        let jd = JulianDate::<TT>::new(2_451_545.0);
        let dbg = format!("{jd:?}");
        assert!(dbg.contains("TT"), "debug should contain scale name");
        assert!(dbg.contains("JD"), "debug should contain format name");
    }

    #[test]
    fn jd_on_tt_and_utc_are_distinct_types() {
        fn accept_tt(x: JulianDate<TT>) -> Day {
            x.raw()
        }
        fn accept_utc(x: JulianDate<UTC>) -> Result<Day, crate::ConversionError> {
            x.try_raw_with(&TimeContext::new())
        }

        let tt_jd = JulianDate::<TT>::new(2_451_545.0);
        let utc_jd =
            JulianDate::<UTC>::try_new_with(Day::new(2_451_545.0), &TimeContext::new()).unwrap();

        let _ = accept_tt(tt_jd);
        assert!(accept_utc(utc_jd).is_ok());
    }

    #[test]
    fn format_names_are_correct() {
        assert_eq!(JD::NAME, "JD");
        assert_eq!(MJD::NAME, "MJD");
        assert_eq!(J2000s::NAME, "J2000s");
        assert_eq!(Unix::NAME, "Unix");
        assert_eq!(GPS::NAME, "GPS");
    }

    #[test]
    fn utc_jd_mjd_use_civil_days_and_roundtrip() {
        let ctx = TimeContext::new();
        let cases = [
            (51_544.0, 946_684_800_i64),
            (57_753.0, 1_483_142_400_i64),
            (57_754.0, 1_483_228_800_i64),
            (61_771.25, 1_830_319_200_i64),
        ];
        for (mjd, unix) in cases {
            let utc = crate::Time::<UTC, MJD>::try_new_with(Day::new(mjd), &ctx).unwrap();
            let label = utc.try_to_chrono_with(&ctx).unwrap();
            assert!((label.timestamp() - unix).abs() <= 1);

            let back = utc
                .to_with::<MJD>(&ctx)
                .unwrap()
                .try_raw_with(&ctx)
                .unwrap();
            assert!((back.value() - mjd).abs() < 1e-9, "MJD {mjd}: {back}");

            let jd =
                crate::Time::<UTC, JD>::try_new_with(Day::new(mjd + 2_400_000.5), &ctx).unwrap();
            let jd_back = jd.to_with::<JD>(&ctx).unwrap().try_raw_with(&ctx).unwrap();
            assert!((jd_back.value() - (mjd + 2_400_000.5)).abs() < 1e-9);
            assert_eq!(utc.to_j2000s(), jd.to_j2000s());
            assert!((jd_back.value() - back.value() - 2_400_000.5).abs() < 1e-9);
        }
        let utc_2017 = crate::Time::<UTC, MJD>::try_new_with(Day::new(57_754.0), &ctx).unwrap();
        let tai = utc_2017.to::<TAI>();
        assert_eq!(
            tai.to_j2000s().raw_seconds_pair(),
            utc_2017.to_j2000s().raw_seconds_pair()
        );
        let tai_mjd = tai.to::<MJD>().raw();
        assert!((tai_mjd.value() - (57_754.0 + 37.0 / 86_400.0)).abs() < 1e-9);
    }

    #[test]
    fn utc_jd_mjd_handle_leap_boundary_without_aliasing() {
        let ctx = TimeContext::new();
        let before =
            crate::Time::<UTC, MJD>::try_new_with(Day::new(57_753.0 + 86_399.5 / 86_400.0), &ctx)
                .unwrap();
        assert_eq!(
            before.try_to_chrono_with(&ctx).unwrap().timestamp(),
            1_483_228_799
        );
        let midnight = crate::Time::<UTC, MJD>::try_new_with(Day::new(57_754.0), &ctx).unwrap();
        assert_eq!(
            midnight.try_to_chrono_with(&ctx).unwrap().timestamp(),
            1_483_228_800
        );

        let leap_label = ::chrono::DateTime::parse_from_rfc3339("2016-12-31T23:59:60Z")
            .unwrap()
            .with_timezone(&::chrono::Utc);
        let leap = crate::Time::<UTC>::try_from_chrono(leap_label).unwrap();
        assert!(matches!(
            leap.to_with::<MJD>(&ctx),
            Err(crate::ConversionError::InvalidLeapSecond)
        ));
        assert!(matches!(
            leap.to_with::<JD>(&ctx),
            Err(crate::ConversionError::InvalidLeapSecond)
        ));
    }

    #[test]
    fn utc_jd_mjd_reject_nonfinite_and_obey_predefinition_policy() {
        let ctx = TimeContext::new();
        assert!(matches!(
            crate::Time::<UTC, MJD>::try_new_with(Day::new(f64::NAN), &ctx),
            Err(crate::ConversionError::NonFinite)
        ));
        assert!(matches!(
            crate::Time::<UTC, JD>::try_new_with(Day::new(f64::INFINITY), &ctx),
            Err(crate::ConversionError::NonFinite)
        ));
        assert!(matches!(
            crate::Time::<UTC, MJD>::try_new_with(Day::new(f64::MAX), &ctx),
            Err(crate::ConversionError::NonFinite)
        ));
        assert!(matches!(
            crate::Time::<UTC, MJD>::try_new_with(Day::new(37_299.0), &ctx),
            Err(crate::ConversionError::UtcBeforeDefinition)
        ));
        let permissive = ctx.allow_pre_definition_utc();
        let old = crate::Time::<UTC, MJD>::try_new_with(Day::new(37_299.0), &permissive).unwrap();
        assert!((old.try_raw_with(&permissive).unwrap().value() - 37_299.0).abs() < 1e-9);
    }

    #[test]
    fn chrono_helpers_with_explicit_context_cover_tt_encoded_formats() {
        let ctx = TimeContext::new().allow_pre_definition_utc();
        let dt =
            ::chrono::DateTime::<::chrono::Utc>::from_timestamp(946_728_123, 250_000_000).unwrap();

        let jd = JulianDate::<TT>::try_from_chrono_with(dt, &ctx).unwrap();
        let mjd = ModifiedJulianDate::<TT>::from_chrono_with(dt, &ctx);
        let j2k = J2000Seconds::<TT>::from(dt);

        let jd_back = jd.try_to_chrono_with(&ctx).unwrap();
        let mjd_back = mjd.to_chrono_with(&ctx).unwrap();
        let j2k_back = j2k.to_chrono().unwrap();

        assert!(
            (jd_back.timestamp_nanos_opt().unwrap() - dt.timestamp_nanos_opt().unwrap()).abs()
                < 50_000
        );
        assert!(
            (mjd_back.timestamp_nanos_opt().unwrap() - dt.timestamp_nanos_opt().unwrap()).abs()
                < 50_000
        );
        assert!(
            (j2k_back.timestamp_nanos_opt().unwrap() - dt.timestamp_nanos_opt().unwrap()).abs()
                < 50_000
        );
    }

    #[test]
    fn format_trait_impls_cover_j2000_jd_mjd_and_gps() {
        let ctx = TimeContext::new();
        let tt = J2000Seconds::<TT>::new(123.5);
        let tai = crate::Time::<TAI>::new(456.75);

        let j2000 = <J2000s as crate::format::FormatForScale<TT>>::try_from_time(tt, &ctx).unwrap();
        assert_eq!(j2000, tt.raw());
        assert_eq!(
            <J2000s as crate::format::FormatForScale<TT>>::try_into_time(j2000, &ctx).unwrap(),
            tt
        );

        let jd = <JD as crate::format::FormatForScale<TT>>::try_from_time(tt, &ctx).unwrap();
        let mjd = <MJD as crate::format::FormatForScale<TT>>::try_from_time(tt, &ctx).unwrap();
        assert!(
            (<JD as crate::format::FormatForScale<TT>>::try_into_time(jd, &ctx)
                .unwrap()
                .to_j2000s()
                .raw()
                .value()
                - tt.raw().value())
            .abs()
                < 1e-4
        );
        assert!(
            (<MJD as crate::format::FormatForScale<TT>>::try_into_time(mjd, &ctx)
                .unwrap()
                .to_j2000s()
                .raw()
                .value()
                - tt.raw().value())
            .abs()
                < 1e-4
        );

        let gps = <GPS as crate::format::FormatForScale<TAI>>::try_from_time(tai, &ctx).unwrap();
        assert_eq!(
            <GPS as crate::format::FormatForScale<TAI>>::try_into_time(gps, &ctx).unwrap(),
            tai.to::<GPS>()
        );
    }
}
