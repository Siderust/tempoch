// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Serde round-trip examples for `Time<S, F>` and `Period<S, F>`.
//!
//! Run with:
//! ```sh
//! cargo run -p tempoch --example serde --features serde
//! ```

use qtty::Second;
use tempoch::{Mjd, Period, Time, TT, UTC, UnixSecs};

fn main() {
    let tt = Time::<TT>::from_si_seconds(Second::new(42.5)).unwrap();
    let unix = Time::<UTC, UnixSecs>::from(1_700_000_000_i64);
    let window = Period::<TT, Mjd>::new(61_000.0, 61_001.0);

    // `Time<S, F>` serializes as the raw format value.
    let tt_json = serde_json::to_string(&tt).unwrap();
    let unix_json = serde_json::to_string(&unix).unwrap();

    // `Period<S, F>` serializes as an object with `start` and `end`.
    let window_json = serde_json::to_string(&window).unwrap();

    println!("TT JSON           : {tt_json}");
    println!("Unix JSON         : {unix_json}");
    println!("Period JSON       : {window_json}");

    let tt_back: Time<TT> = serde_json::from_str(&tt_json).unwrap();
    let unix_back: Time<UTC, UnixSecs> = serde_json::from_str(&unix_json).unwrap();
    let window_back: Period<TT, Mjd> = serde_json::from_str(&window_json).unwrap();

    assert_eq!(tt_back, tt);
    assert_eq!(unix_back, unix);
    assert_eq!(window_back, window);

    println!("TT round-trip     : {:.1}", tt_back.si_seconds());
    println!("Unix round-trip   : {}", unix_back.value().value());
    println!(
        "Period round-trip : {:.1} -> {:.1} MJD",
        window_back.start.modified_julian_days().value(),
        window_back.end.modified_julian_days().value()
    );
}
