All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Replaced the `archive/` git submodule with a regular crates.io-style git
  dependency on `siderust-archive` (sourced via `[patch.crates-io]` from
  `https://github.com/Siderust/archive.git` until the crate is published).
- `tempoch-time-data` now depends on `siderust-archive` with only the `time`
  feature (or `time` + `fetch` when `tempoch-time-data/fetch` is enabled).
  Runtime download (`TimeDataManager`) continues to work behind the
  `runtime-data-fetch` feature on `tempoch-core`.

### Removed

- Deleted `archive/` git submodule (and `.gitmodules`). Downstream consumers
  no longer need `git submodule update --init --recursive`.
- Deleted `tempoch-time-data-updater` crate. Maintenance of the IERS dataset
  has moved to the archive repository, where the
  `siderust-archive-update-time-data` binary plus the
  `update-time-data.yml` GitHub Actions workflow regenerate the bundle and
  cut a new patch release on crates.io.
- Deleted `.github/workflows/update-time-data.yml` and the supporting scripts
  in `.github/scripts/` (no longer needed; weekly refresh runs in the archive
  repo instead).

### Changed

- `tempoch-time-data`: migrated to thin re-export of `siderust-archive`
  (was `siderust-archive-data` from the archive submodule). The embedded
  5.3 MB IERS EOP array (`eop_data.rs`) had already been removed in a prior
  patch; this release completes the migration by also removing the submodule.

## [0.6.3] - 2026-05-28

### Fixed

- `tempoch-time-data`: marked `publish = false` in `Cargo.toml`. The crate was
  already published at 0.1.3 on crates.io; the publish-changed script was
  attempting to re-publish the same version on every workspace release because the
  root `Cargo.toml` change triggered all packages to be considered changed.
  Dependents (`tempoch-core`) continue to resolve it via the registry version field.

## [0.6.2] - 2026-05-28

### Added

- Added `ExactDuration`, an opaque signed nanosecond duration backed by `i128` with range ≈ ±5.4 × 10²¹ yr. Provides three boundary-projection variants: `as_seconds_i64_nanos_checked` (exact, returns `Err(DurationError::Overflow)` when the seconds component does not fit in `i64`), `as_seconds_i64_nanos_saturating` (documented lossy for extreme values), and `as_seconds_i64_nanos` (panics on overflow). Also provides `from_canonical_seconds_nanos` (requires `|nanos| < 1_000_000_000` and matching signs when `seconds ≠ 0`) plus explicit lossy `f64` conversion, quantum-based rounding, `qtty` conversion, and serde support as `{"sec": i64, "ns": i32}`.
- Added `DurationError::NonCanonical` variant returned by `from_canonical_seconds_nanos` when the signs of `seconds` and `nanos` do not match.
- Added `try_add_exact` / `try_sub_exact` fallible variants on `Time<S, F>` that return `Err(DurationError)` when the duration's seconds component exceeds the `i64` range. The infallible `add_exact` / `sub_exact` delegate to these and panic on overflow; no silent saturation occurs.
- Added `ExactDuration::from_nanoseconds_i(qtty::i64::Nanosecond)` and `as_nanoseconds_i(self) -> Result<qtty::i64::Nanosecond, DurationError>` for typed integer nanosecond access without going through `f64`.
- Added `ExactDuration::from_seconds_i(qtty::i64::Second)` for whole-second construction via a typed quantity.
- Added `GnssWeek::subsecond_nanoseconds_u() -> qtty::u32::Nanosecond` and `GnssWeek::seconds_of_week_u() -> qtty::u32::Second` typed read accessors, and `GnssWeek::new_with_nanoseconds_u(week, seconds_of_week, qtty::u32::Nanosecond)` typed constructor. Unsigned quantities are used because week, seconds-of-week, and subsecond nanoseconds are non-negative fields.
- Added exact-duration integration for `Time<S, F>`: ExactDuration-based difference and add/subtract helpers with split-f64 precision limits documented; epoch-relative round/floor/ceil helpers. Existing subtraction behavior is preserved for compatibility.
- Added new sealed time-scale markers: `ET`, `GPST`, `GST`, `BDT`, and `QZSST`, including GNSS reference validation data.
- Added `TimeSeries<S, F>`, a time iterator with exact integer-nanosecond step scheduling and deterministic by-index construction (no accumulated repeated-add drift); produced `Time<S>` values retain split-f64 precision limits.
- Added `tempoch-validation` as a non-published workspace crate with reference datasets and provenance metadata.
- Added property tests for exact-duration invariants, rounding behavior, scale-conversion round trips, and exact time add/subtract behavior.
- Added native ISO 8601 / RFC 3339 parser/formatter for `Time<UTC>` (`parse_rfc3339`, `format_rfc3339`) with:
  - Leap-second-aware **parsing** of the `23:59:60[.x]` form, validated against the compiled UTC-TAI table.
  - Leap-second-aware **formatting**: `format_rfc3339_with` / `try_format_rfc3339_with` emit `23:59:60[.fraction]Z` for instants inside an announced positive leap-second window. Detection uses `Time<UTC>::is_leap_second_with(ctx)`, which consults the compiled UTC–TAI table, as the authoritative signal.
  - Configurable subsecond precision (0–9 digits) with `Truncate` / `RoundHalfToEven` rounding applied uniformly, including correct carry overflow into the next second.
  - Parser hardening: empty fractional part (e.g. `12:34:56.Z`) and more than 9 fractional digits are rejected.
  - Fallible formatting API `try_format_rfc3339_with(...) -> Result<String, ConversionError>`; the infallible `format_rfc3339_with` delegates to it and returns `"<invalid>"` on error (documented).
- Added GNSS week / seconds-of-week format (`GnssWeek` + `GnssWeekScale` trait) implemented for `GPST`, `GST`, `BDT`, and `QZSST`, with documented rollover periods (1024 / 4096 / 8192 / 1024) and overflow detection in `to_gnss_week` (returns `ConversionError::OutOfRange` for week numbers exceeding `u32::MAX`).
- Added `tempoch_core::data::provenance` with a programmatic `ProvenanceSnapshot` (source URLs, SHA-256, validity horizons), and an `assert_fresh(now, max_age)` freshness checker exposed at the crate root as `time_data_provenance()` and `assert_time_data_fresh(...)`.

### Fixed

- `ExactDuration` serde `Serialize` now returns an error if the stored value exceeds the `i64` seconds range; previously the boundary projection silently saturated.
- `ExactDuration::from_quantity` now panics unconditionally on non-finite or overflowing input in all build profiles; the previous release-mode silent-fallback path has been removed.
- `ExactDuration::round_to`, `floor_to`, and `ceil_to` now use saturating arithmetic throughout to prevent overflow on extreme (`i128::MIN/MAX`) inputs.
- RFC 3339 parser now validates `:60` leap-second inputs against the compiled UTC-TAI table; dates that were not announced leap seconds (e.g. `2023-06-15T23:59:60Z`) now return `ConversionError::InvalidLeapSecond` instead of being accepted.
- RFC 3339 parser now rejects empty fractional parts (e.g. `2024-06-15T12:34:56.Z`) and inputs with more than 9 fractional digits; previously these were silently accepted or truncated.
- RFC 3339 formatter now applies `FormatPrecision::Truncate` and `FormatPrecision::RoundHalfToEven` uniformly for all `subsecond_digits` in 0–9, including correct carry overflow at `.999999999` into the next second.
- `Time::add_exact` and `sub_exact` now panic on overflow instead of silently saturating when the `ExactDuration`'s seconds component exceeds the `i64` range. New fallible `try_add_exact` / `try_sub_exact` variants return `Err(DurationError::Overflow)` in that case. Both use a two-step split-arithmetic path: the `ExactDuration` is decomposed into whole-second and nanosecond-remainder components, each added to the split-f64 storage separately. This avoids collapsing the full duration to a single `f64` before addition, preserving sub-millisecond precision for typical astronomical epochs.
- `Time::diff_exact` documentation now accurately states that the result is bounded by split-f64 storage precision (~100–150 ns near J2000 ± 50 years), not sub-nanosecond fidelity for arbitrary instants.
- `Time::to_gnss_week` now uses integer arithmetic on the split-f64 storage pair: the integer-second component is extracted and subtracted from the (exact-integer) epoch constant in `i128`, and the subsecond remainder is computed from the fractional part alone. This eliminates the catastrophic cancellation that previously occurred when subtracting the epoch from the total J2000 seconds in a single `f64`.
- `Time::from_gnss_week` now constructs the epoch `Time<S>` and calls `add_exact` with the exact `ExactDuration` since epoch, instead of collapsing total nanoseconds to `f64` before adding the epoch offset. Nanosecond fields are preserved to within split-f64 storage precision (≤ 200 ns for typical GNSS epochs).
- GNSS round-trip tests now validate `subsecond_nanos` within ±200 ns and require exact `seconds_of_week` matching; the previous test allowed ≤1 second drift and did not validate subsecond fields.
- `render_with_digits` (non-standard subsecond digit counts 1, 2, 4, 5, 7, 8) now carries rounding overflow into the seconds field; previously a round-up at `.999...` would produce a digit count exceeding the requested width.

### Changed

- Extended the scale-conversion matrix to cover the new ET and GNSS scales alongside existing TAI, TT, TDB, TCG, TCB, UT1, and UTC routes.
- Documented shipped accuracy classes, validation status, and remaining roadmap items without referencing external comparison projects.

### Roadmap

- Still pending: `no_std` split for `tempoch-core`, CCSDS time-code parsers, formal-verification (Kani) harnesses, and FFI/WASM/Python ABI updates.


## [0.6.1] - 2026-05-25

### Changed

- Bumped workspace and crate versions to `0.6.2` for a patch release. Aligned
  `qtty` dependency to `0.8.4` in `tempoch-time-data` and synchronized the
  `tempoch-ffi` ABI line to `0.6.2 -> 602`.


### Changed

- `tempoch-ffi`: marked `publish = false` in `Cargo.toml`. FFI crates are not
  published by default; publish only when C API/ABI changes are intentional.
  See `tempoch-ffi/README.md` for the manual publish procedure.
- `tempoch-ffi`: replaced publishing-blocking unsafe-block TODO markers with
  explicit `SAFETY` rationales for caller-provided output pointers.


## [0.6.0] - 2026-05-18

### Added

- `From`/`Into` from `JulianDate<S>`, `ModifiedJulianDate<S>`, `UnixTime`, and `GpsTime` into default-tagged `Time<S>` / `Time<UTC>` / `Time<TAI>` (same instant as `Time::to_j2000s`), so `Period::try_new` / `Interval::try_new` accept encoded endpoints directly.
- `Interval::length<U>()` for requesting interval durations in any `qtty` time unit supported by the endpoint difference type, with `Interval::duration<U>()` kept as the backward-compatible alias.
- `Time<TT, JD>::JD_EPOCH_J2000_0` plus the typed epoch helpers in `tempoch::constats` / `tempoch_core::foundation::constats` (`j2000_jd_tt`, `unix_epoch_jd`, `gps_epoch_tai`, `tdb_tt_model_high_accuracy_*`, and related day/second constants).

### Breaking

- Removed `Time::to_time` and `Time::to_time_with`; use `Time::to_j2000s()` for the canonical
  `Time<S, J2000s>` tag (`to_time_with` ignored context except for API shape).
- Removed `Coord<S, F>` and `Offset<S, F>` from `tempoch-core`. Epochs are either
  `Time<S, F>` via helpers in `foundation::constats` / `tempoch::constats`
  (`j2000_jd_tt`, `unix_epoch_jd`, …) or bare `qtty::Day` / `qtty::Second`
  constants (`J2000_JD_TT_DAY`, `GPS_EPOCH_TAI_SECONDS`, …). Replace
  `J2000_JD_TT.raw()` with `J2000_JD_TT_DAY` or `j2000_jd_tt().raw()`.
- `DELTA_T_PREDICTION_HORIZON_MJD` is now a plain `qtty::Day` (same numeric value
  as before); drop `.raw()` when comparing against other `Day` quantities.
- Removed `tempoch::format`, `tempoch::period`, and `tempoch::features` sub-module
  shim paths from the `tempoch` facade crate. All public types are available at
  the crate root (`tempoch::JD`, `tempoch::Period`, etc.). Users who depended on
  `tempoch::format::*` or `tempoch::period::*` paths should migrate to the flat
  root or depend on `tempoch-core` directly.

### Changed

- `Time<S, F>` is now the canonical storage model throughout `tempoch-core`; the format tag is a typed external view over the same split J2000-second storage rather than a separate storage representation.
- The crate layout was reorganized around `foundation`, `model`, `earth`, `period`, and `features`, with the generated bundled tables living under `tempoch-time-data`.
- README and numbered examples were updated to use `.to_j2000s()`, `Into<Time<_>>`, and the new typed epoch helpers instead of the removed `to_time*` helpers and older constant forms.

### Internal

- Format marker types (`JD`, `MJD`, `J2000s`, `Unix`, `GPS`) moved from
  `tempoch_core::encoding` to `tempoch_core::format::markers`; the public
  surface at `tempoch_core::format::{JD, MJD, …}` is unchanged.
- `format/mod.rs` split into focused sub-files (`traits.rs`, `encoded_time.rs`,
  `impls.rs`, `markers.rs`).
- `period/mod.rs` error types extracted to `period/error.rs`.

## [0.5.1] - 2026-05-17

### Changed
- Some FFI entries were outdated and have been corrected.

- Refreshed generated time tables (fetched 2026-05-18):
  - Earth Orientation Parameters finals2000A (IERS)

## [0.5.0] - 2026-05-17

### Breaking

- Renamed `TimeRepresentation` → `TimeFormat`; canonical module is now
  `format` (was `representation`).
- Renamed `RepresentationForScale<S>` → `FormatForScale<S>`.
- Renamed `InfallibleRepresentationForScale<S>` →
  `InfallibleFormatForScale<S>`.
- Swapped type parameter order on `Coord` and `Offset`:
  `Coord<F, S>` / `Offset<F, S>` → `Coord<S, F>` / `Offset<S, F>`. The
  new order (Scale first, Format second) matches `EncodedTime<S, F>` and
  makes type signatures read in the natural order "JD on TT" →
  `Coord<TT, JD>`.
- Updated all constants in `tempoch::constats` to the new parameter order:
  `J2000_JD_TT: Coord<TT, JD>`, `GPS_EPOCH_JD_TAI: Coord<TAI, JD>`, etc.

### Changed

- All doc comments now use "format" (not "representation") for the encoding
  phantom-type axis.
- `Coord` module documentation updated to clarify the `<S, F>` ordering
  convention.
- `tempoch_core::format` is now the canonical public module for format
  traits and markers.

## [0.4.5] - 2026-05-11

### Changed

- Refreshed generated time tables (fetched 2026-05-11):
  - Earth Orientation Parameters finals2000A (IERS)

## [0.4.4] - 2026-05-09

### Changed

- ffi params refactor unix -> unix_ts

### Removed

- **BREAKING:** Removed deprecated `JulianTimeExt` trait and all its method implementations. All methods have been available as inherent methods on `JulianDate<S>` and `ModifiedJulianDate<S>` since version 0.4.4:
  * `.jd_value()`, `.mjd_value()` on `EncodedTime`, `JulianDate`, `ModifiedJulianDate`
  * `.julian_centuries()`, `.julian_millennias()` on encoded time types
  * `.quantity()` on encoded time types
  * `.min()`, `.max()`, `.mean()` on encoded time types
- **BREAKING:** Removed deprecated `EncodedTime<S, JD>::value()` alias (use `.jd_value()` or `.raw().value()`).
- **BREAKING:** Removed deprecated `EncodedTime<S, MJD>::value()` alias (use `.mjd_value()` or `.raw().value()`).
- **BREAKING:** Removed deprecated `JulianDate::from_utc()` and `to_utc()` (use `from_chrono()` and `to_chrono()`).
- **BREAKING:** Removed deprecated `ModifiedJulianDate::from_utc()` and `to_utc()` (use `from_chrono()` and `to_chrono()`).

### Fixed

* Resolved all `cargo doc --no-deps` intra-doc link warnings (10 warnings → 0):
  * `context.rs` — fixed `Time::<UTC>::try_from_chrono_with` link to use `crate::` prefix.
  * `delta_t.rs` — fixed `Time::to_scale_with::<UT1>` link to use `crate::` prefix.
  * `eop.rs` — fixed `EOP_OBSERVED_END_MJD` link to use `crate::` prefix; removed
    broken link to private `crate::generated::eop_data`.
  * `error.rs` — replaced broken links to feature-gated `update_runtime_time_data` /
    `refresh_runtime_time_data` with plain prose.
  * `scale/mod.rs` — rewrote UTC "Authoritative UTC API" section, removing references
    to non-existent `from_unix_seconds` / `unix_seconds` methods and replacing them with
    the correct `try_to::<Unix>()` API; fixed all `Time::` links to use `crate::` prefix.



## [0.4.3 - 2026-05-04]

### Changed

- Updated release dependencies to `affn 0.6.2`, `qtty 0.7.0`, and `qtty-ffi 0.7.0`.
- Refreshed README install snippets for the `0.4.4` line.

## [0.4.2 - 2026-04-25]

### Added

- The new axis/representation time model now backs the public API:
  `Time<A, R = Native>` with sealed axis markers (`TAI`, `TT`, `TDB`, `TCG`,
  `TCB`, `UTC`, `UT1`), representation markers (`Native`, `JulianDays`,
  `ModifiedJulianDays`, `SISeconds`, `UnixSeconds<POSIX>`, `GpsSeconds`),
  unified conversion witness traits, explicit `TimeContext`, and generic
  `Interval<T>` / `Period<S>` helpers.
- Optional `serde` support on `tempoch-core` and `tempoch`, including
  tagged wrappers `tempoch::tagged::{TaggedTime, TaggedPeriod}` for wire
  formats that must carry the scale name in-band.
- Runtime refresh/update support, backed by the new internal
  `tempoch-time-data` support crate, the public refresh/update entrypoints,
  and the `tempoch-time-data-updater` CLI plus scheduled refresh workflow.
- Bundled daily IERS Earth Orientation Parameters under `tempoch::eop`,
  `TimeContext::with_builtin_eop()`, and the public
  `EOP_START_MJD` / `EOP_OBSERVED_END_MJD` / `EOP_END_MJD` coverage constants.
- New numbered examples covering quickstart, scale conversions, coordinate
  views, periods, `serde`, runtime tables, and mixed conversion workflows.
- FFI additions for the new core model: `TEMPOCH_STATUS_T_UT1_HORIZON_EXCEEDED`,
  leap-second round-tripping with `second = 60`, and Unix timestamp identity
  helpers `tempoch_unix_from_seconds` / `tempoch_unix_to_seconds`.

### Removed

- Deleted the old `Time<S>` compatibility layer entirely, including the
  `tempoch::legacy` / `tempoch_core::legacy` namespaces.
- Dropped the legacy-only `serde` feature from `tempoch-core`, `tempoch`, and
  `tempoch-ffi`.
- Removed obsolete updater-local parser modules
  (`tempoch-time-data-updater/src/parse.rs` and
  `tempoch-time-data-updater/src/eop.rs`); parsing now lives only in
  `tempoch-time-data`.

### Fixed

- `Interval::complement` now clips gaps to the outer interval when a sorted
  input period starts after the outer end.
- Removed a spurious Kepler-equation correction from the dominant
  Fairhead-Bretagnon TDB-TT term, eliminating an approximately 28 µs
  systematic error.
- `tempoch-ffi` now keeps its UTC civil-time behavior aligned with the Rust
  crate's supported pre-1961 approximate continuation instead of documenting
  those dates as rejected.
- Post-horizon Delta T extrapolation now caches its quadratic tail-fit
  coefficients with `OnceLock` instead of recomputing them for every call.
- Removed dead `f64::EPSILON` exact-match checks from the modern Delta T
  interpolator.

### Changed

- The public façade now centers the scale-only `Time<S>` model, with
  coordinate and transport encodings exposed as conversion targets (`JD`,
  `MJD`, `J2000s`, `Unix`, `GPS`) instead of storage types.
- UTC and UT1 behavior now comes from generated official UTC-TAI, Delta T,
  and EOP tables. `TimeContext::new()` remains monthly-Delta-T by default,
  `with_builtin_eop()` prefers the bundled daily DUT1 path in range, and
  pre-1961 UTC civil labels continue through the documented approximate
  extension.
- Runtime data no longer exposes a parallel public API. The normal chrono,
  Unix, and context-backed conversion entrypoints now consult the lazily
  selected active bundle, while refresh/cache management stays internal.
- Unix and GPS transport helpers now use typed `qtty::Second`, and
  `DELTA_T_PREDICTION_HORIZON_MJD` is now exported as a typed `qtty::Day`.
- `tai_minus_utc()` now uses the official pre-1972 UTC frequency-offset
  history from 1961 onward while preserving the documented 10 s fallback for
  earlier dates.
- `TCB` conversions now compose the linear `TCB <-> TDB` relation with the
  existing periodic `TDB <-> TT` correction, removing the previous
  millisecond-scale `TDB <-> TCB` round-trip drift.
- `tempoch-time-data` is now publishable as the internal support crate used by
  `tempoch-core`, so the `0.4.2` release graph can be dry-run and published
  directly from crates.io-resolved dependencies.
- `tempoch-ffi` now maps UT1 horizon failures to a dedicated status code and
  keeps its Unix and civil-time conversions aligned with the crate's leap-second
  and pre-1961 UTC semantics.
- Updated release dependencies to `affn 0.6.1`, `qtty 0.6.1`, and
  `qtty-ffi 0.6.1`; workspace CI now checks all feature combinations used by
  the release validation path and gates coverage on the public runtime/FFI
  crates while still running the maintenance-crate tests.

## [0.4.1 - 2026-03-31]

### Changed

- Simplified `tempoch-ffi` to a scalar C ABI centered on `double` time values plus raw scale IDs, with generic `tempoch_time_convert`, `tempoch_time_from_utc`, `tempoch_time_to_utc`, `tempoch_time_difference_*`, and `tempoch_time_add_*` entrypoints.
- Standardized the FFI `UnixTime` contract as POSIX seconds since `1970-01-01T00:00:00 UTC`, instead of exposing the Rust crate's internal day-count representation directly.
- Bumped `tempoch-ffi` to `0.4.0` / ABI version `400` and regenerated the public C header to match the new generic surface.
- Simplified the `tempoch-cpp` wrapper to rebuild typed `Time<S>` operations on top of the generic scale-based FFI calls rather than maintaining a hand-written pairwise conversion matrix.

### Fixed

- `tempoch-ffi` period APIs now reject malformed `MJD` intervals consistently, including non-finite endpoints passed across the C boundary.
- Rust and C++ FFI-facing civil-time docs now agree on the supported `second` range (`0..=59`), removing the earlier leap-second mismatch at the adapter boundary.

## [0.4.0 - 2026-03-08]

### Added

- Typed `qtty-ffi` duration helpers in `tempoch-ffi`, including `tempoch_jd_difference_qty`, `tempoch_jd_add_qty`, `tempoch_mjd_difference_qty`, `tempoch_mjd_add_qty`, `tempoch_jd_julian_centuries_qty`, and `tempoch_period_mjd_duration_qty`.
- FFI time-scale conversion functions for `TDB`, `TT`, `TAI`, `TCG`, `TCB`, `GPS`, `UT`, `JDE`, and `UnixTime`.
- FFI `TempochScale` enum plus generic `tempoch_jd_to_scale` / `tempoch_scale_to_jd` dispatch helpers.
- `timescales` example covering the supported scale conversions.

### Changed

- Updated the public `qtty` dependency to `0.4.1`.
- `Time<S>` display output now omits the redundant trailing day-unit suffix and forwards standard formatting flags to the raw numeric value.
- Generated FFI headers now include `qtty_ffi.h` instead of duplicating shared quantity declarations.
- Improved formatting and consistency in FFI time conversion implementations.

### Fixed

- `Interval<DateTime<Utc>>::duration_days()` now preserves sub-second precision instead of truncating through whole seconds.

## [0.3.0 - 2026-02-19]

### Added

- `Time::try_new` and `Time::try_from_days` validated constructors that reject `NaN`/`±∞`.
- `Interval::try_new` validated constructor that rejects `start > end` (and `NaN` endpoints).
- `NonFiniteTimeError`, `InvalidIntervalError`, `PeriodListError` error types.
- `validate_period_list` — checks sorted/non-overlapping invariants on a period slice.
- `normalize_periods` — sorts and merges overlapping intervals into a valid list.
- FFI: generated C header is now also written to `tempoch-ffi/include/tempoch_ffi.h`.

### Changed

- `Time::new` and `Interval::new` now carry documentation warnings about
  accepting unchecked input; prefer the new `try_*` constructors for
  untrusted data.
- Serde deserialization of `Time<S>`, `Period<MJD>`, and `Period<JD>` now
  rejects non-finite values and invalid intervals (start > end).
- `Time::<JD>::tt_to_tdb` now delegates to the shared `tdb_minus_tt_days`
  function (previously duplicated the Fairhead & Bretagnon math).

### Fixed

- Eliminated duplicated TDB correction logic between `scales.rs` and
  `julian_date_ext.rs`, reducing drift risk.

## [0.2.1 - 2026-02-19]

### Added

- FFI support

### Fixed

- Preserve sub-second precision in `Interval<DateTime<Utc>>::duration_days()` by computing from nanoseconds instead of truncating via whole seconds.

## [0.2.0 - 2026-02-16]

### Added

- New coordinate time scales: `TCG` (Geocentric Coordinate Time) and `TCB` (Barycentric Coordinate Time).
- Crate-root exports for `TCG` and `TCB` (`tempoch::TCG`, `tempoch::TCB`).
- Additional conversion tests covering `TDB`, `TCG`, `TCB`, and leap-second-aware Unix conversions.

### Changed

- `TDB` conversions no longer treat `TT` as strictly identical; they now apply periodic Fairhead & Bretagnon correction terms.
- `UnixTime` conversions now apply leap-second-aware UTC/TAI/TT offsets from an IERS Bulletin C table (1972–2017).
- `ΔT` now uses observed annual values for 1992–2025 and linear near-term extrapolation after 2026, replacing the prior modern-year approximation.

## [0.1.0 - 2026-02-12]

### Added

- Initial standalone release extracted from `siderust::time`.
- Generic time instants and scales via `Time<S>` and `TimeScale`.
- Marker scales: `JD`, `JDE`, `MJD`, `TDB`, `TT`, `TAI`, `GPS`, `UnixTime`, `UT`.
- `JulianDate`, `JulianEphemerisDay`, `ModifiedJulianDate`, `UniversalTime` aliases.
- UTC conversion helpers (`from_utc`, `to_utc`) using `chrono`.
- Automatic `ΔT` correction layer for `UT` conversions.
- Generic intervals (`Interval<T>`) and scale-based periods (`Period<S>`).
- Set-like period utilities: `intersect_periods` and `complement_within`.
- GitHub Actions CI workflow with check/fmt/clippy/test/doctest/coverage jobs.
- Runnable examples and integration tests.
