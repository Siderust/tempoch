# tempoch

[![Crates.io](https://img.shields.io/crates/v/tempoch.svg)](https://crates.io/crates/tempoch)
[![Docs](https://docs.rs/tempoch/badge.svg)](https://docs.rs/tempoch)
[![Code Quality](https://github.com/Siderust/tempoch/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Siderust/tempoch/actions/workflows/ci.yml)

Typed astronomical time primitives for Rust.

`tempoch` is built around a few deliberate modeling choices:

- A `Time<S>` value is an instant on a scale-specific axis, not a bare `f64`.
- Time instants are modeled with affine semantics: `time_a - time_b` yields a
  duration, while shifting an instant is `time + seconds`.
- The canonical internal carrier is a split `(hi, lo)` pair of J2000-based
  seconds so epoch-sized values can retain sub-second precision through
  conversions and arithmetic.
- `JD`, `MJD`, `J2000s`, `Unix`, and `GPS` are views or transport
  encodings, not alternate storage backends.
- `UTC` keeps its civil meaning, but internally it is stored on a continuous
  instant axis and interpreted through the active UTC-TAI data tables.

`tempoch` provides:

- `Time<S>` instants parameterized by a physical or civil scale (`TT`,
  `TAI`, `UTC`, `UT1`, `TDB`, `TCG`, `TCB`).
- Unified target-based conversions:
  - `.to::<TT>()`, `.to::<UTC>()`, `.to::<TDB>()` for infallible scale routes
  - `.to_with::<UT1>(&ctx)` for context-backed UT1 routes, or `.try_to::<UT1>()` shorthand (snapshots active data at call time)
  - `.to::<JD>()`, `.to::<MJD>()`, `.to::<J2000s>()` for coordinate views
  - `.try_to::<Unix>()` and `.to::<GPS>()` for transport encodings
- UTC conversion through `chrono`, leap-second aware over the official history
  (1961-01-01 onward). Requests for dates before the UTC standard was defined
  return `ConversionError::UtcBeforeDefinition` by default; opt in to the
  approximate segment back-extrapolation by building your context with
  `TimeContext::new().allow_pre_definition_utc()`.
- Automatic `ΔT = TT - UT1` handling for `UT1` conversions via an explicit
  `TimeContext`. For the currently compiled bundle fetched 2026-04-18, the
  default monthly-ΔT path stays within 15 ms of the bundled daily IERS-derived
  path over the observed overlap, and within 0.2 s over the compiled
  short-range prediction overlap. Opt into
  `TimeContext::with_builtin_eop()` when you want the highest-fidelity bundled
  UT1 path; raw EOP values are available under `tempoch::eop` and bracketed by
  the public `EOP_START_MJD` / `EOP_OBSERVED_END_MJD` / `EOP_END_MJD`
  constants.
- TT↔TDB conversion via the built-in seven-term Fairhead–Bretagnon
  approximation from USNO Circular 179. The crate documents about 10 µs
  accuracy only inside the public
  `constats::TDB_TT_MODEL_HIGH_ACCURACY_START_JD` →
  `constats::TDB_TT_MODEL_HIGH_ACCURACY_END_JD` interval (about 1600-01-01 to
  2200-01-01 TT).
- Julian Day, Modified Julian Day, and SI-second views via `JD`, `MJD`, and
  `J2000s` conversion targets on every built-in scale, including UTC's stored
  instant axis.
- Unix/POSIX timestamps via `UnixTime::try_new(sec).and_then(|e| e.to_time_with(&ctx))` and `.try_to::<Unix>()`.
- GPS transport values via `GpsTime::try_new(sec).map(|e| e.to_time())` and `.to::<GPS>()`.
- Compiled time-data tables generated from official UTC-TAI and Delta T
  sources.
- Optional `serde` support for `Time<S>` as `{"hi","lo"}` and
  `Period<S>` / `Interval<T>` as `{start, end}` objects, plus explicit
  `tempoch::tagged::{TaggedTime, TaggedPeriod}` wrappers when the payload must
  carry the scale name.
- Automatic runtime freshness backed by a cached time-data bundle, while
  keeping the same public API.
- Public typed epoch/offset constants under `tempoch::constats`, such as
  `J2000_JD_TT`, `TT_MINUS_TAI`, and `DELTA_T_PREDICTION_HORIZON_MJD`.
- A utility `Interval<T>` type for half-open time ranges over `Time<A>`,
  with intersection, normalization, validation, and complement helpers.

## Compiled Tables And Official References

The generated tables under `tempoch-core/src/generated/` are tied to explicit
authoritative sources:

| Generated table | Purpose | Official reference | Canonical upstream |
|---|---|---|---|
| `UTC_TAI_SEGMENTS` | UTC-to-TAI history, including pre-1972 rate segments and post-1972 leap-second steps | IERS Bulletin C / `UTC-TAI.history` | `https://hpiers.obspm.fr/eoppc/bul/bulc/UTC-TAI.history` |
| `MODERN_DELTA_T_POINTS` | Compiled modern `ΔT = TT - UT1` series | USNO observed monthly `deltat.data` plus USNO predicted `deltat.preds` | `https://maia.usno.navy.mil/ser7/deltat.data` and `https://maia.usno.navy.mil/ser7/deltat.preds` |
| `EOP_POINTS` | Daily Earth-orientation parameters used by the bundled UT1/EOP path | IERS combined Bulletin A + C04 file `finals2000A.all` | `https://datacenter.iers.org/data/9/finals2000A.all` |

`MODERN_DELTA_T_POINTS` is a derived bundle in this crate: the observed USNO
monthly series is concatenated with C0-adjusted USNO predictions, and the
observed/predicted boundary is exposed through
`MODERN_DELTA_T_OBSERVED_END_MJD`.

**Storage model:** `Time<S>` stores a compensated `(hi, lo)` pair of seconds
since J2000 TT on the target axis. Tags such as `JD`, `MJD`, `Unix`, and
`GPS` are conversion targets, not storage types.

## Design Decisions

### `Time<S>` is an affine point, not a scalar

`tempoch` treats an instant as a point on a time axis. That is why the API is
deliberately shaped around:

- `time_a - time_b -> duration`
- `time + duration -> time`
- `time - duration -> time`

and does not model "adding two instants" as a meaningful operation.

Internally, this is represented with `affn::SplitPoint1`, which keeps the
affine semantics aligned with the rest of the geometry model instead of
re-introducing scalar-like mistakes through ad hoc time arithmetic.

### Why time uses split storage

Astronomical epochs are large in absolute value, but many important corrections
are tiny:

- leap-second boundaries
- TT-TAI offsets
- TT-TDB corrections
- UT1 adjustments
- sub-second transport roundtrips

Using one `f64` for "seconds since epoch" would lose low-order precision once a
small correction is combined with a value on the order of `1e9` seconds.
`tempoch` therefore stores every `Time<S>` as a normalized compensated pair
`(hi, lo)` whose sum is the represented instant. The high part carries the
epoch-sized component; the low part preserves the small remainder.

This is why `Time<S>` serializes as `{"hi","lo"}` with `serde`, and why the
crate reuses `affn::SplitQuantity` / `SplitPoint1` instead of a plain scalar
field.

### Why J2000 seconds are canonical

The crate exposes Julian Day, Modified Julian Day, Unix, and GPS helpers, but
those are public coordinate or transport views. The storage choice remains
canonical J2000-based seconds because it gives one precise internal axis for
arithmetic and scale conversion, while still allowing JD/MJD-style APIs at the
boundary.

### Format vs Scale

Two orthogonal phantom-type axes drive all typed time values in `tempoch`:

- **Scale** — the physical time axis: `TT`, `TAI`, `UTC`, `UT1`, `TDB`,
  `TCG`, `TCB`. A scale tells you *which* reference frame's clock is ticking.
- **Format** — how an instant is numerically encoded: `JD` (Julian Day),
  `MJD` (Modified Julian Day), `J2000s` (SI seconds since J2000), `Unix`
  (POSIX seconds), `GPS` (days since GPS epoch). A format tells you *how* you
  read the number off the wire.

The two are always kept separate:

- [`Time<S>`] stores no format; it is a raw J2000-based compensated pair on
  scale `S`.
- [`EncodedTime<S, F>`] (and its aliases `JulianDate<S>`, `ModifiedJulianDate<S>`,
  etc.) pair a scale with a format for I/O and coordinate constants.
- [`Coord<S, F>`] is the typed low-level counterpart used for compile-time
  constants (`J2000_JD_TT: Coord<TT, JD>`, etc.).

The type parameter order is always **Scale first, Format second** — matching
the reading direction "JD on TT" → `Coord<TT, JD>` / `EncodedTime<TT, JD>`.

`ScaleKind` variant names follow the same `<Format><Scale>` pattern:
`JdTt`, `MjdTt`, `JdTdb`, `JdTai`, `JdTcg`, `JdTcb`, `JdGps`, `JdUt1`,
`Unix`.



`UTC` is a civil scale with leap-second labeling, so it cannot be treated as a
simple uniform scalar everywhere. `tempoch` keeps the internal instant storage
continuous, then maps that stored instant to civil UTC through the active
UTC-TAI table. That separation lets the crate support both:

- precise instant arithmetic and transport encodings
- leap-second-aware civil conversions

The compiled modern ΔT series runs through MJD 63871 (`2033-10-01`). Beyond
that date the built-in bundle stops and UT1 conversions fail with
`ConversionError::Ut1HorizonExceeded` unless an active runtime bundle extends
the horizon. Use the exported `DELTA_T_PREDICTION_HORIZON_MJD` typed
`qtty::Day` constant to reference the compiled boundary programmatically.

## Installation

```toml
[dependencies]
tempoch = "0.5.0"
```

Enable `serde` if you want to serialize typed times and periods:

```toml
[dependencies]
tempoch = { version = "0.5.0", features = ["serde"] }
```

The `serde` feature composes with the ordinary runtime refresh behavior:

```toml
[dependencies]
tempoch = { version = "0.5.0", features = ["serde", "runtime-data-fetch"] }
```

## Serde

With the `serde` feature enabled:

- `Time<S>` serializes as `{"hi": ..., "lo": ...}`.
- `Period<S>` serializes as `{"start": ..., "end": ...}`.
- The scale remains type-level and is not embedded in the payload.
- `tagged::TaggedTime<S>` and `tagged::TaggedPeriod<S>` serialize with an
  explicit `"scale"` field for interchange payloads.

```rust
use qtty::Second;
use tempoch::{
    tagged::{TaggedPeriod, TaggedTime},
    J2000Seconds, Period, TT,
};

let tt = J2000Seconds::<TT>::try_new(Second::new(42.5)).unwrap().to_time();
let period = Period::<TT>::new(
    tt,
    J2000Seconds::<TT>::try_new(Second::new(43.5)).unwrap().to_time(),
);

assert_eq!(serde_json::to_string(&tt).unwrap(), r#"{"hi":42.5,"lo":0.0}"#);
assert_eq!(
    serde_json::to_string(&period).unwrap(),
    r#"{"start":{"hi":42.5,"lo":0.0},"end":{"hi":43.5,"lo":0.0}}"#
);
assert_eq!(
    serde_json::to_string(&TaggedTime(tt)).unwrap(),
    r#"{"scale":"TT","hi":42.5,"lo":0.0}"#
);
assert_eq!(
    serde_json::to_string(&TaggedPeriod(period)).unwrap(),
    r#"{"scale":"TT","start":{"scale":"TT","hi":42.5,"lo":0.0},"end":{"scale":"TT","hi":43.5,"lo":0.0}}"#
);
```

## Quick Start

```rust
use chrono::Utc;
use tempoch::{JD, MJD, Time, TT, UTC};

let utc_now = Time::<UTC>::from_chrono(Utc::now());
let tt_now: Time<TT> = utc_now.to::<TT>();

println!("UTC       : {}", utc_now.to_chrono().unwrap());
println!("TT in JD  : {:.9}", tt_now.to::<JD>());
println!("TT in MJD : {:.9}", tt_now.to::<MJD>());
```

## Period Operations

```rust
use qtty::Day;
use tempoch::{ModifiedJulianDate, Period, TT};

let day = Period::<TT>::new(
  ModifiedJulianDate::<TT>::try_new(Day::new(61_000.0)).unwrap().to_time(),
  ModifiedJulianDate::<TT>::try_new(Day::new(61_001.0)).unwrap().to_time(),
);
let a = vec![
  Period::<TT>::new(
    ModifiedJulianDate::<TT>::try_new(Day::new(61_000.1)).unwrap().to_time(),
    ModifiedJulianDate::<TT>::try_new(Day::new(61_000.4)).unwrap().to_time(),
  ),
  Period::<TT>::new(
    ModifiedJulianDate::<TT>::try_new(Day::new(61_000.6)).unwrap().to_time(),
    ModifiedJulianDate::<TT>::try_new(Day::new(61_000.9)).unwrap().to_time(),
  ),
];
let b = vec![
  Period::<TT>::new(
    ModifiedJulianDate::<TT>::try_new(Day::new(61_000.2)).unwrap().to_time(),
    ModifiedJulianDate::<TT>::try_new(Day::new(61_000.3)).unwrap().to_time(),
  ),
  Period::<TT>::new(
    ModifiedJulianDate::<TT>::try_new(Day::new(61_000.7)).unwrap().to_time(),
    ModifiedJulianDate::<TT>::try_new(Day::new(61_000.8)).unwrap().to_time(),
  ),
];

let overlap = Period::intersect_many(&a, &b);
let gaps = day.complement(&a);

assert_eq!(overlap.len(), 2);
assert_eq!(gaps.len(), 3);
```

## Examples

- `cargo run --example 01_quickstart`
- `cargo run --example 02_scales`
- `cargo run --example 03_formats`
- `cargo run --example 04_periods`
- `cargo run --example 05_serde --features serde`
- `cargo run -p tempoch --example 06_runtime_tables`
- `cargo run --example 07_conversions`

## Runtime Time Data

`tempoch` automatically prefers a cached runtime bundle for fresher UTC-TAI
history, modern Delta T, and daily IERS EOP while keeping the public API
unchanged. `TimeContext` and `Time::to_with` consult a cached bundle in
`~/.tempoch/data`, refreshing it once on first use when the cache is missing,
invalid, or older than 24 hours. Data-dependent shorthand methods (e.g.
`try_to::<UT1>()`, `try_to::<Unix>()`, `try_to_chrono()`) snapshot a fresh
`TimeContext` internally. For reproducible pipelines, use the `_with` variants
with an explicit context.

Set `TEMPOCH_DATA_DIR` to override the cache location.

For a runnable example that uses the ordinary API with runtime refresh, run:

```bash
cargo run -p tempoch --example 06_runtime_tables
```

```rust,no_run
use qtty::{Day, Second};
use tempoch::{JD, JulianDate, Time, TimeContext, UnixTime, Unix, TT, UT1, UTC};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = TimeContext::with_builtin_eop();
    let tt = JulianDate::<TT>::try_new(Day::new(2_460_000.25))?.to_time();
    let ut1: Time<UT1> = tt.to_with::<UT1>(&ctx)?;

    let unix = UnixTime::try_new(Second::new(1_700_000_000.0))
        .and_then(|e| e.to_time_with(&ctx))?;
    let back = unix.try_to::<Unix>()?;

    println!("UT1 JD     : {:.9}", ut1.to::<JD>());
    println!("Unix roundtrip: {:.3}", back);
    Ok(())
}
```

## Time Data Updates

The compile-time path still uses checked-in generated tables in `tempoch-core`.
The dedicated Rust CLI `tempoch-time-data-updater` regenerates those committed
files from the official UTC-TAI, Delta T, and IERS finals2000A.all sources.
Its fetch/parse/build pipeline now reuses the same shared support crate that
powers runtime refresh. The updater intentionally keeps only render/write
orchestration; parser and bundle-building logic is centralized in
`tempoch-time-data` to avoid runtime/compile-time drift. To refresh locally:

```bash
cargo run -p tempoch-time-data-updater
cargo test --all-features
```

To verify manually that the committed generated files are still in sync with
upstream:

```bash
cargo run -p tempoch-time-data-updater -- --check
```

A scheduled GitHub Actions workflow runs the refresh automatically every
Monday at 05:23 UTC and pushes the resulting commit directly to `main` when
the generated tables or their source hashes change. GitHub cron schedules are
defined in UTC.

## Tests and Coverage

```bash
cargo test --all-targets
cargo test --doc
cargo +nightly llvm-cov --workspace --all-features --doctests --summary-only
```

Coverage is gated in CI at **>= 90% line coverage**.

## License

AGPL-3.0-only
