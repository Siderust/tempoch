# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.4 - 2026-05-09]

### Changed

- ffi params refactor unix -> unix_ts

## [0.4.3 - 2026-05-04]

### Changed

- Updated release dependencies to `affn 0.6.2`, `qtty 0.7.0`, and `qtty-ffi 0.7.0`.
- Refreshed README install snippets for the `0.4.3` line.

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
