# tempoch

[![Crates.io](https://img.shields.io/crates/v/tempoch.svg)](https://crates.io/crates/tempoch)
[![Docs](https://docs.rs/tempoch/badge.svg)](https://docs.rs/tempoch)
[![Code Quality](https://github.com/Siderust/tempoch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Siderust/tempoch/actions/workflows/ci.yml)

Typed astronomical time primitives for Rust.

`tempoch` provides:

- `Time<A>` instants parameterized by a physical or civil axis (`TT`, `TAI`,
  `UTC`, `UT1`, `TDB`, `TCG`, `TCB`).
- Compile-time conversion witnesses for infallible (`to`) and
  context-required (`to_with`) routes.
- UTC conversion through `chrono`, exact from 1961 onward and leap-second
  aware.
- Automatic `ΔT = TT - UT1` handling for `UT1` conversions via an explicit
  `TimeContext`.
- Julian Day, Modified Julian Day, and SI-second accessors as direct methods
  on `Time<A>` for continuous axes.
- Unix/POSIX timestamps via `Time::<UTC>::from_unix_seconds` / `unix_seconds`.
- GPS transport values via `Time::<TAI>::from_gps_seconds` / `gps_seconds`.
- Compiled time-data tables generated from official UTC-TAI and Delta T
  sources.
- Public typed epoch/offset constants under `tempoch::constats`, such as
  `J2000_JD_TT`, `TT_MINUS_TAI`, and `DELTA_T_PREDICTION_HORIZON_MJD`.
- A utility `Interval<T>` type for half-open time ranges over `Time<A>`,
  with intersection, normalization, validation, and complement helpers.

**Storage model:** `Time<A>` stores a single `f64` second count since J2000 TT
on the target axis. Precision therefore depends on the epoch magnitude; around
contemporary dates the floor is sub-microsecond, but it degrades as the
absolute second count grows.

The compiled modern ΔT series runs through MJD 63871 (`2033-10-01`). Beyond
that date UT1 conversions fail with `ConversionError::Ut1HorizonExceeded`.
Use the exported `DELTA_T_PREDICTION_HORIZON_MJD` typed `qtty::Day` constant
to reference the compiled boundary programmatically.

## Installation

```toml
[dependencies]
tempoch = "0.4"
```

## Quick Start

```rust
use chrono::Utc;
use tempoch::{Time, TT, UTC};

let utc_now = Time::<UTC>::from_chrono(Utc::now());
let tt_now = utc_now.to::<TT>();

println!("UTC: {}", utc_now.to_chrono().unwrap());
println!("JD(TT):  {:.9}", tt_now.julian_days());
println!("MJD(TT): {:.9}", tt_now.modified_julian_days());
```

## Period Operations

```rust
use qtty::Day;
use tempoch::{complement_within, intersect_periods, Interval, Time, TT};

fn mjd(value: f64) -> Time<TT> {
    Time::<TT>::from_modified_julian_days(Day::new(value)).unwrap()
}

let outer = Interval::new(mjd(0.0), mjd(10.0));
let a = vec![
    Interval::new(mjd(1.0), mjd(4.0)),
    Interval::new(mjd(6.0), mjd(9.0)),
];
let b = vec![
    Interval::new(mjd(2.0), mjd(3.0)),
    Interval::new(mjd(7.0), mjd(8.0)),
];

let overlap = intersect_periods(&a, &b);
let gaps = complement_within(outer, &a);

assert_eq!(overlap.len(), 2);
assert_eq!(gaps.len(), 3);
```

## Examples

- `cargo run --example quickstart`
- `cargo run --example periods`
- `cargo run --example timescales`

## Time Data Updates

The crate compiles generated time-data tables into `tempoch-core`, rather than
fetching them at runtime. To refresh the checked-in data locally:

```bash
python3 scripts/update_time_data.py
cargo test --all-features
```

A scheduled GitHub Actions workflow also runs this refresh automatically and
opens a pull request when the generated tables change.

## Tests and Coverage

```bash
cargo test --all-targets
cargo test --doc
cargo +nightly llvm-cov --workspace --all-features --doctests --summary-only
```

Coverage is gated in CI at **>= 90% line coverage**.

## License

AGPL-3.0-only
