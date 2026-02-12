# tempoch

[![Crates.io](https://img.shields.io/crates/v/tempoch.svg)](https://crates.io/crates/tempoch)
[![Docs](https://docs.rs/tempoch/badge.svg)](https://docs.rs/tempoch)
[![Code Quality](https://github.com/Siderust/tempoch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Siderust/tempoch/actions/workflows/ci.yml)

Typed astronomical time primitives for Rust.

`tempoch` provides:

- Generic `Time<S>` instants parameterized by time-scale markers (`JD`, `MJD`, `TT`, `UT`, `TAI`, `GPS`, `UnixTime`, ...).
- Built-in UTC conversion through `chrono`.
- Automatic `ΔT = TT - UT` handling for the `UT` scale.
- Generic intervals with `Interval<T>` and scale-aware alias `Period<S>`.
- Utility operations like period intersection and complement.

## Installation

```toml
[dependencies]
tempoch = "0.1"
```

## Quick Start

```rust
use chrono::Utc;
use tempoch::{JulianDate, MJD, Time};

let now_jd = JulianDate::from_utc(Utc::now());
let now_mjd: Time<MJD> = now_jd.to::<MJD>();

println!("JD(TT): {now_jd}");
println!("MJD(TT): {now_mjd}");
```

## Period Operations

```rust
use tempoch::{complement_within, intersect_periods, ModifiedJulianDate, Period};

let outer = Period::new(ModifiedJulianDate::new(0.0), ModifiedJulianDate::new(10.0));
let a = vec![
    Period::new(ModifiedJulianDate::new(1.0), ModifiedJulianDate::new(4.0)),
    Period::new(ModifiedJulianDate::new(6.0), ModifiedJulianDate::new(9.0)),
];
let b = vec![
    Period::new(ModifiedJulianDate::new(2.0), ModifiedJulianDate::new(3.0)),
    Period::new(ModifiedJulianDate::new(7.0), ModifiedJulianDate::new(8.0)),
];

let overlap = intersect_periods(&a, &b);
let gaps = complement_within(outer, &a);

assert_eq!(overlap.len(), 2);
assert_eq!(gaps.len(), 3);
```

## Examples

- `cargo run --example quickstart`
- `cargo run --example periods`

## Tests and Coverage

```bash
cargo test --all-targets
cargo test --doc
cargo +nightly llvm-cov --workspace --all-features --doctests --summary-only
```

Coverage is gated in CI at **>= 90% line coverage**.

## License

AGPL-3.0-only
