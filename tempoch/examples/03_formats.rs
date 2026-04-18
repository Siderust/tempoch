// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Format conversion showcase: construct and re-encode `Time<S, F>` values.
//!
//! Sections:
//!   1. SI seconds (J2000-TT offset) baseline on TT
//!   2. Julian Day / Modified Julian Day conversions on TT
//!   3. POSIX / Unix seconds round-trip on UTC
//!   4. GPS seconds round-trip on TAI
//!   5. chrono `DateTime<Utc>` round-trip on UTC
//!
//! Run with:
//! ```sh
//! cargo run -p tempoch --example formats
//! ```

use chrono::Utc;
use qtty::{Day, Second};
use tempoch::{
    constats::{J2000_JD_TT, UNIX_EPOCH_JD, UNIX_EPOCH_MJD},
    Jd, Mjd, Time, TAI, TT, UTC,
};

fn main() {
    // ─────────────────────────────────────────────────────────────────────────
    // 1. SI seconds (J2000-TT offset)
    //
    // The default format for `Time<S>`. Zero corresponds to J2000.0:
    // 2000-01-01T12:00:00 TT. Negative values are before J2000; positive
    // after.
    // ─────────────────────────────────────────────────────────────────────────
    let j2000_tt = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
    let sample_tt = Time::<TT>::from_si_seconds(Second::new(123_456.789)).unwrap();

    println!("── 1. SI seconds (J2000-TT offset) ───────────────────────────");
    println!("  J2000             : {j2000_tt:.3}");
    println!("  Sample TT         : {sample_tt:.6}");

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Julian Day (JD) and Modified Julian Day (MJD)
    //
    // Reformat preserves the physical instant and only changes representation.
    // ─────────────────────────────────────────────────────────────────────────
    let j2000_from_jd = Time::<TT, Jd>::from_julian_days(J2000_JD_TT).unwrap();
    let unix_epoch_jd = Time::<TT, Jd>::from_julian_days(UNIX_EPOCH_JD).unwrap();
    let half_day_jd = Time::<TT, Jd>::from_julian_days(Day::new(2_451_545.5)).unwrap();
    let unix_epoch_mjd = Time::<TT, Mjd>::from_modified_julian_days(UNIX_EPOCH_MJD).unwrap();

    let sample_as_jd: Time<TT, Jd> = sample_tt.reformat();
    let sample_as_mjd: Time<TT, Mjd> = sample_tt.reformat();
    let sample_roundtrip: Time<TT> = sample_as_mjd.reformat();

    println!();
    println!("── 2. JD / MJD conversions (TT) ──────────────────────────────");
    println!("  J2000 JD(TT)      : {j2000_from_jd:.9}");
    println!("  Unix epoch JD(TT) : {unix_epoch_jd:.9}");
    println!("  Unix epoch MJD(TT): {unix_epoch_mjd:.9}");
    println!("  JD + 0.5 (noon)   : {half_day_jd:.9}");
    println!("  Sample as JD      : {sample_as_jd:.9}");
    println!("  Sample as MJD     : {sample_as_mjd:.9}");
    println!(
        "  SI round-trip err : {:.3e} s",
        (sample_roundtrip - sample_tt).value()
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 3. POSIX / Unix seconds (UTC scale only)
    //
    // Counts seconds from 1970-01-01T00:00:00 UTC, ignoring leap seconds.
    // ─────────────────────────────────────────────────────────────────────────
    let unix_epoch_utc = Time::<UTC>::from_unix_seconds(Second::new(0.0)).unwrap();
    let y2024_unix = Second::new(1_704_067_200.0);
    let y2024_utc = Time::<UTC>::from_unix_seconds(y2024_unix).unwrap();
    let y2024_unix_roundtrip = y2024_utc.unix_seconds().unwrap();

    println!();
    println!("── 3. POSIX / Unix seconds (UTC) ─────────────────────────────");
    println!("  Unix epoch        : {:.3}", unix_epoch_utc.unix_seconds().unwrap());
    println!("  2024-01-01 UTC    : {:.3}", y2024_unix_roundtrip);
    println!(
        "  Unix round-trip err: {:.3e} s",
        (y2024_unix_roundtrip - y2024_unix).value()
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 4. GPS seconds (TAI scale only)
    //
    // Counts seconds from the GPS epoch: 1980-01-06T00:00:00 UTC.
    // GPS = TAI − 19 s (exact, fixed at launch).
    // ─────────────────────────────────────────────────────────────────────────
    let gps_input = Second::new(1_000_000.0);
    let gps_tai = Time::<TAI>::from_gps_seconds(gps_input).unwrap();
    let gps_roundtrip = gps_tai.gps_seconds();

    println!();
    println!("── 4. GPS seconds (TAI) ──────────────────────────────────────");
    println!("  Input GPS seconds : {:.3}", gps_input);
    println!("  Output GPS seconds: {:.3}", gps_roundtrip);
    println!(
        "  GPS round-trip err: {:.3e} s",
        (gps_roundtrip - gps_input).value()
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 5. chrono DateTime<Utc> (UTC scale only)
    //
    // Full wall-clock string interop via the `chrono` crate.
    // ─────────────────────────────────────────────────────────────────────────
    let dt_str = "2000-01-01T00:00:00Z";
    let dt = dt_str.parse::<chrono::DateTime<Utc>>().unwrap();
    let from_str = Time::<UTC>::try_from_chrono(dt).unwrap();
    let now_utc = Time::<UTC>::from_chrono(Utc::now());

    println!();
    println!("── 5. chrono DateTime<Utc> (UTC) ─────────────────────────────");
    println!("  Parsed from RFC3339 : {}", from_str.to_chrono().unwrap());
    println!(
        "  Now (RFC3339 ms)    : {}",
        now_utc
            .to_chrono()
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    );
    println!("  Is leap second      : {}", now_utc.is_leap_second());
}
