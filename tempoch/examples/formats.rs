// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Interchange format showcase: every way to construct and read a `Time<S, F>`.
//!
//! Sections:
//!   1. SI seconds (J2000-TT offset) — J2000s format, all continuous scales
//!   2. Julian Day — Jd format, all continuous scales
//!   3. Modified Julian Day — Mjd format, all continuous scales
//!   4. POSIX / Unix seconds — UTC scale only
//!   5. GPS seconds — TAI scale only
//!   6. chrono `DateTime<Utc>` — UTC scale only
//!
//! Run with:
//! ```sh
//! cargo run -p tempoch --example formats
//! ```

use chrono::Utc;
use qtty::{Day, Second};
use tempoch::{
    constats::{GPS_EPOCH_TAI, J2000_JD_TT, UNIX_EPOCH_JD, UNIX_EPOCH_MJD},
    Jd, Mjd, Time, TAI, TT, UTC,
};

fn main() {
    // ─────────────────────────────────────────────────────────────────────────
    // 1. SI seconds (J2000-TT offset)
    //
    // The default format for `Time<S>`. Zero corresponds to J2000.0:
    // 2000-01-01T12:00:00 TT. Negative values are before J2000; positive
    // after. Available on all continuous scales.
    // ─────────────────────────────────────────────────────────────────────────
    let j2000_tt = Time::<TT>::from_si_seconds(Second::new(0.0)).unwrap();
    let one_day_later = Time::<TT>::from_si_seconds(Second::new(86_400.0)).unwrap();
    let one_year_before = Time::<TT>::from_si_seconds(Second::new(-365.25 * 86_400.0)).unwrap();

    println!("── 1. SI seconds (J2000-TT offset) ───────────────────────────");
    println!("  J2000             : {:.3}", j2000_tt.si_seconds());
    println!("  J2000 + 1 day     : {:.3}", one_day_later.si_seconds());
    println!("  J2000 − 1 year    : {:.3}", one_year_before.si_seconds());
    println!("  Elapsed (b − a)   : {:.3}", one_day_later - j2000_tt);

    // ─────────────────────────────────────────────────────────────────────────
    // 2. Julian Day (JD)
    //
    // Continuous day count from the Julian epoch (noon, 1 Jan 4713 BC).
    // JD = 2 451 545.0 at J2000.0. Fractions denote time-of-day.
    // Uses the Jd format: `Time<TT, Jd>`.
    // ─────────────────────────────────────────────────────────────────────────
    let j2000_from_jd = Time::<TT, Jd>::from_julian_days(J2000_JD_TT).unwrap();
    let unix_epoch_jd = Time::<TT, Jd>::from_julian_days(UNIX_EPOCH_JD).unwrap();
    let half_day_jd = Time::<TT, Jd>::from_julian_days(Day::new(2_451_545.5)).unwrap();

    println!();
    println!("── 2. Julian Day ─────────────────────────────────────────────");
    println!("  J2000 JD(TT)   : {:.9}", j2000_from_jd.julian_days());
    println!("  Unix epoch JD  : {:.9}", unix_epoch_jd.julian_days());
    println!("  JD + 0.5 (noon): {:.9}", half_day_jd.julian_days());
    // Reformat to SI seconds to show the round-trip
    let j2000_roundtrip: Time<TT> = j2000_from_jd.reformat();
    println!("  Round-trip SI  : {:.3}", j2000_roundtrip.si_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // 3. Modified Julian Day (MJD)
    //
    // MJD = JD − 2 400 000.5. Starts at midnight on 1858-11-17.
    // Uses the Mjd format: `Time<TT, Mjd>`.
    // ─────────────────────────────────────────────────────────────────────────
    let j2000_from_mjd = Time::<TT, Mjd>::from_modified_julian_days(Day::new(51_544.5)).unwrap();
    let unix_epoch_mjd = Time::<TT, Mjd>::from_modified_julian_days(UNIX_EPOCH_MJD).unwrap();

    println!();
    println!("── 3. Modified Julian Day (MJD) ──────────────────────────────");
    println!(
        "  J2000 MJD(TT)  : {:.9}",
        j2000_from_mjd.modified_julian_days()
    );
    println!(
        "  Unix epoch MJD : {:.9}",
        unix_epoch_mjd.modified_julian_days()
    );
    // Cross-format comparison: reformat JD to MJD
    let j2000_as_mjd: Time<TT, Mjd> = j2000_from_jd.reformat();
    println!(
        "  JD − MJD       : {:.1}",
        j2000_from_jd.julian_days() - j2000_as_mjd.modified_julian_days()
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 4. POSIX / Unix seconds (UTC scale only)
    //
    // Counts seconds from 1970-01-01T00:00:00 UTC, ignoring leap seconds.
    // ─────────────────────────────────────────────────────────────────────────
    let unix_epoch_utc = Time::<UTC>::from_unix_seconds(Second::new(0.0)).unwrap();
    let y2024_utc = Time::<UTC>::from_unix_seconds(Second::new(1_704_067_200.0)).unwrap();
    let now_utc = Time::<UTC>::from_chrono(Utc::now());

    println!();
    println!("── 4. POSIX / Unix seconds (UTC) ─────────────────────────────");
    println!(
        "  Unix epoch       : {:.3}",
        unix_epoch_utc.unix_seconds().unwrap()
    );
    println!(
        "  2024-01-01 UTC   : {:.3}",
        y2024_utc.unix_seconds().unwrap()
    );
    println!(
        "  Now              : {:.3}",
        now_utc.unix_seconds().unwrap()
    );
    println!(
        "  Reconstructed UTC: {}",
        unix_epoch_utc.to_chrono().unwrap()
    );

    // ─────────────────────────────────────────────────────────────────────────
    // 5. GPS seconds (TAI scale only)
    //
    // Counts seconds from the GPS epoch: 1980-01-06T00:00:00 UTC.
    // GPS = TAI − 19 s (exact, fixed at launch).
    // ─────────────────────────────────────────────────────────────────────────
    let gps_epoch_tai = Time::<TAI>::from_gps_seconds(Second::new(0.0)).unwrap();
    let gps_j2000: Time<TAI> = j2000_from_jd.reformat::<tempoch::J2000s>().to_scale();
    let now_gps: Time<TAI> = now_utc.to_scale();

    println!();
    println!("── 5. GPS seconds (TAI) ──────────────────────────────────────");
    println!("  GPS epoch TAI offset : {:.3}", GPS_EPOCH_TAI);
    println!(
        "  GPS epoch gps_seconds: {:.3}",
        gps_epoch_tai.gps_seconds()
    );
    println!("  J2000 GPS seconds    : {:.3}", gps_j2000.gps_seconds());
    println!("  Now GPS seconds      : {:.3}", now_gps.gps_seconds());

    // ─────────────────────────────────────────────────────────────────────────
    // 6. chrono DateTime<Utc> (UTC scale only)
    //
    // Full wall-clock string interop via the `chrono` crate.
    // ─────────────────────────────────────────────────────────────────────────
    let dt_str = "2000-01-01T00:00:00Z";
    let dt = dt_str.parse::<chrono::DateTime<Utc>>().unwrap();
    let from_str = Time::<UTC>::try_from_chrono(dt).unwrap();

    println!();
    println!("── 6. chrono DateTime<Utc> (UTC) ─────────────────────────────");
    println!("  Parsed from RFC3339 : {}", from_str.to_chrono().unwrap());
    println!(
        "  Now (RFC3339 ms)    : {}",
        now_utc
            .to_chrono()
            .unwrap()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    );
    println!("  Is leap second      : {}", now_utc.is_leap_second());

    // Round-trip: UTC → TAI → UTC should preserve the wall-clock instant.
    let now_roundtrip: Time<UTC> = now_utc.to_scale::<TAI>().to_scale();
    let orig = now_utc.to_chrono().unwrap();
    let trip = now_roundtrip.to_chrono().unwrap();
    println!(
        "  UTC→TAI→UTC delta   : {:.0} ns",
        (orig - trip).num_nanoseconds().unwrap_or(0)
    );
}
