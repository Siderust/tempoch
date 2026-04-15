# tempoch

[![Crates.io](https://img.shields.io/crates/v/tempoch.svg)](https://crates.io/crates/tempoch)
[![Docs](https://docs.rs/tempoch/badge.svg)](https://docs.rs/tempoch)
[![Code Quality](https://github.com/Siderust/tempoch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Siderust/tempoch/actions/workflows/ci.yml)

Typed astronomical time primitives for Rust.

`tempoch` provides:

- `Time<A, R = Native>` instants with a first-class separation between
  physical/civil axes (`TT`, `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`) and
  representations (`JulianDays`, `ModifiedJulianDays`, `SISeconds`,
  `UnixSeconds<POSIX>`, `GpsSeconds`).
- Compile-time conversion witnesses for infallible, fallible, and
  context-required conversions.
- UTC conversion through `chrono`, exact from 1961 onward and leap-second
  aware.
- Automatic `ΔT = TT - UT1` handling for `UT1` conversions through an
  explicit `TimeContext`.
- Standard Unix/POSIX timestamps via `UnixSeconds<POSIX>` and GPS transport
  values via `GpsSeconds`.
- Compiled time-data tables generated from official UTC-TAI and Delta T
  sources.
- Generic intervals with `Interval<T>` plus utility operations like
  intersection, normalization, validation, and complement.

**Storage model:** `Time<A, R>` stores a single `f64` second count since
J2000 TT on the target axis. Precision therefore depends on the epoch
magnitude; around contemporary dates the floor is sub-microsecond, but it
still degrades as the absolute second count grows.

The compiled modern ΔT series runs through MJD 63871 (`2033-10-01`).  Beyond
that date UT1 conversions fail with `ConversionError::Ut1HorizonExceeded`.
Use the exported `DELTA_T_PREDICTION_HORIZON_MJD` constant to reference the
compiled boundary programmatically.

## Installation

```toml
[dependencies]
tempoch = "0.4"
```

## Quick Start

```rust
use chrono::Utc;
use tempoch::{JulianDays, ModifiedJulianDays, Time, TT, UTC};

let utc_now = Time::<UTC>::from_chrono(Utc::now());
let tt_now = utc_now.to::<TT>();
let jd_tt: Time<TT, JulianDays> = tt_now.repr();
let mjd_tt: Time<TT, ModifiedJulianDays> = tt_now.repr();

println!("UTC: {}", utc_now.to_chrono().unwrap());
println!("JD(TT): {}", jd_tt.julian_days().value());
println!("MJD(TT): {}", mjd_tt.modified_julian_days().value());
```

## Period Operations

```rust
use qtty::Day;
use tempoch::{complement_within, intersect_periods, Interval, ModifiedJulianDays, Time, TT};

type MjdTt = Time<TT, ModifiedJulianDays>;

fn mjd(value: f64) -> MjdTt {
    Time::<TT, ModifiedJulianDays>::from_modified_julian_days(Day::new(value)).unwrap()
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
