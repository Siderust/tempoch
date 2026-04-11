// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Time-scale showcase: construction and conversion between all 11 tempoch scales.
//!
//! Demonstrates:
//! - Constructing typed `Time<S>` values with each scale
//! - Cross-scale conversion via `.to::<S>()`
//! - The J2000.0 reference epoch
//! - `delta_t()` (ΔT = TT − UT1) on a `Time<UT>` value
//! - `tai_minus_utc()` at a UTC-axis JD derived from a UTC instant
//! - A contemporary round-trip through all scales
//!
//! Run with:
//! ```sh
//! cargo run -p tempoch --example timescales
//! ```

use chrono::{DateTime, Utc};
use tempoch::{
    tai_minus_utc, JulianDate, ModifiedJulianDate, Time, GPS, JD, JDE, MJD, TAI, TCB, TCG, TDB, TT,
    UT,
};

fn main() {
    // ── 1. Reference epoch: J2000.0 ─────────────────────────────────────────
    //
    // JD 2 451 545.0 = 2000-01-01T12:00:00 TT
    // In UTC it is about 2000-01-01T11:58:55.816Z because TT−UTC = 64.184 s
    // at that epoch (TAI−UTC = 32 s and TT−TAI = 32.184 s).
    let jd = JulianDate::new(2_451_545.0);
    let utc_j2000 = jd.to_utc().expect("J2000 is representable as UTC");
    let jd_utc = 2_440_587.5
        + (utc_j2000.timestamp() as f64 + utc_j2000.timestamp_subsec_nanos() as f64 / 1e9)
            / 86_400.0;

    println!("Reference: J2000.0");
    println!("  JD value  : {:.6}", jd);
    println!(
        "  UTC instant: {}",
        utc_j2000.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    );
    println!();

    // ── 2. Convert to all scales ─────────────────────────────────────────────
    let mjd: Time<MJD> = jd.to::<MJD>();
    let jde: Time<JDE> = jd.to::<JDE>();
    let tt: Time<TT> = jd.to::<TT>();
    let tai: Time<TAI> = jd.to::<TAI>();
    let tdb: Time<TDB> = jd.to::<TDB>();
    let tcg: Time<TCG> = jd.to::<TCG>();
    let tcb: Time<TCB> = jd.to::<TCB>();
    let gps: Time<GPS> = jd.to::<GPS>();
    let ut: Time<UT> = jd.to::<UT>();

    println!("Scale conversions at J2000.0:");
    println!("  JD           : {:.9}", jd);
    println!("  MJD          : {:.9}", mjd);
    println!("  JDE  (≡ TDB) : {:.9}", jde);
    println!("  TT   (+32.184 s from TAI) : {:.9}", tt);
    println!("  TAI  (TT − 32.184 s)      : {:.9}", tai);
    println!("  TDB  (≈ TT, small periapsis term) : {:.9}", tdb);
    println!("  TCG  (TT + secular drift)  : {:.9}", tcg);
    println!("  TCB  (TDB + secular drift) : {:.9}", tcb);
    println!("  GPS  (TAI − 19 s)          : {:.9}", gps);
    println!("  UT1  (TT − ΔT)             : {:.9}", ut);
    println!();

    // ── 3. ΔT and TAI−UTC offsets ───────────────────────────────────────────
    println!("Offset quantities at J2000.0:");
    let delta_t = ut.delta_t(); // TT − UT1 — already has units in its Display
    println!("  ΔT (TT − UT1) : {}", delta_t);
    let tai_utc = tai_minus_utc(jd_utc); // TAI − UTC in seconds at the UTC instant
    println!("  TAI − UTC     : {} s  (leap seconds)", tai_utc);
    println!();

    // ── 4. Round-trips ──────────────────────────────────────────────────────
    let jd_rt: JulianDate = mjd.to::<JD>();
    println!(
        "Round-trip JD → MJD → JD drift   : {:.3e} days",
        (jd_rt.value() - jd.value()).abs()
    );

    let jd_from_tt: JulianDate = tt.to::<JD>();
    println!(
        "Round-trip JD → TT  → JD drift   : {:.3e} days",
        (jd_from_tt.value() - jd.value()).abs()
    );

    let jd_from_tai: JulianDate = tai.to::<JD>();
    println!(
        "Round-trip JD → TAI → JD drift   : {:.3e} days",
        (jd_from_tai.value() - jd.value()).abs()
    );
    println!();

    // ── 5. Offset between scales (seconds) ──────────────────────────────────
    // TT and TAI are both stored as JD-equivalent days, so direct subtraction
    // is valid.  For TAI-GPS: GPS is stored as days since GPS epoch rather
    // than JD, so we compute the offset from the well-defined formula:
    //   TAI − GPS = (TAI − UTC) − (GPS − UTC)
    // where GPS − UTC = 19 s (constant: GPS was set = TAI at GPS epoch 1980-01-06)
    // and TAI − UTC = tai_minus_utc(jd_utc) (increases with leap seconds).
    const SECONDS_PER_DAY: f64 = 86_400.0;
    const GPS_UTC_OFFSET_S: i32 = 19; // GPS time was locked to TAI at GPS epoch
    let tt_tai_diff_s = (tt.value() - tai.value()) * SECONDS_PER_DAY;
    let tai_utc_s = tai_minus_utc(jd_utc) as i32;
    let tai_gps_diff_s = tai_utc_s - GPS_UTC_OFFSET_S;

    println!("Offsets between scales (seconds):");
    println!("  TT − TAI  : {:.3} s  (fixed 32.184 s)", tt_tai_diff_s);
    println!(
        "  TAI − GPS : {} s  (= TAI−UTC {} s minus GPS−UTC constant 19 s)",
        tai_gps_diff_s, tai_utc_s
    );
    println!();

    // ── 6. Construct directly with typed `new()` constructors ───────────────
    let mjd_direct = ModifiedJulianDate::new(51_544.5); // MJD for J2000.0
    let jd_from_mjd: JulianDate = mjd_direct.to::<JD>();
    println!("MJD 51544.5 → JD: {:.6}", jd_from_mjd);
    println!();

    // ── 7. From UTC datetime ─────────────────────────────────────────────────
    let utc_str = "2026-07-15T22:00:00Z";
    let utc: DateTime<Utc> = utc_str.parse().expect("valid RFC-3339 date");
    let jd_obs = JulianDate::from_utc(utc);
    let mjd_obs: Time<MJD> = jd_obs.to::<MJD>();
    let tt_obs: Time<TT> = jd_obs.to::<TT>();
    let tai_obs: Time<TAI> = jd_obs.to::<TAI>();

    println!("Observation: {utc_str}");
    println!("  JD  : {:.6}", jd_obs);
    println!("  MJD : {:.6}", mjd_obs);
    println!("  TT  : {:.6}", tt_obs);
    println!("  TAI : {:.6}", tai_obs);
    println!(
        "  Julian centuries since J2000: {:.6}",
        jd_obs.julian_centuries()
    );
}
