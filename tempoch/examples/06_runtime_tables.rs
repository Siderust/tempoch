// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Use the ordinary API while `runtime-data` keeps the active bundle fresh.
//!
//! Run with:
//! ```sh
//! cargo run -p tempoch --example 06_runtime_tables --features runtime-data
//! ```

#[cfg(feature = "runtime-data")]
use qtty::{Day, Second};
#[cfg(feature = "runtime-data")]
use tempoch::{Time, TimeContext, JD, TT, UT1, UTC};

#[cfg(not(feature = "runtime-data"))]
fn main() {
    eprintln!("run with: cargo run -p tempoch --example 06_runtime_tables --features runtime-data");
}

#[cfg(feature = "runtime-data")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TimeContext::with_builtin_eop();
    let probe_tt = Time::<TT, JD>::from_julian_days(Day::new(2_460_000.25))?;
    let probe_ut1: Time<UT1, JD> = probe_tt.to_scale_with::<UT1>(&ctx)?;
    let unix_utc = Time::<UTC>::from_unix_seconds(Second::new(1_700_000_000.0))?;
    let unix_roundtrip = unix_utc.unix_seconds()?;

    println!("runtime-data is enabled; first use will prefer a cached bundle and");
    println!("refresh once if the cache is missing, invalid, or older than 24 h.");
    println!("probe TT JD     : {probe_tt:.9}");
    println!("probe UT1 JD    : {probe_ut1:.9}");
    println!("Unix roundtrip  : {:.3}", unix_roundtrip.value());
    Ok(())
}
