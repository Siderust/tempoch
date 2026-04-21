# UTC pre-history is silently extrapolated rather than documented or bounded

## Summary
Querying `Time<UTC>` for dates before the first UTC-TAI table segment
(before 1960-01-01) silently extends the first segment backward using its
linear formula. The result is physically dubious — UTC was only defined from
1960 — but no API-level warning, `None`, or error is returned.

## Status
Resolved. `time_data_try_tai_minus_utc_mjd` now documents the pre-history
extrapolation behavior. `UTC_DEFINED_FROM_MJD` (MJD 37 300, 1961-01-01) is
exported from `tempoch_core` and `tempoch` as a guard boundary for callers.

## What is the issue
The UTC-TAI history table begins at 1960-01-01. The function
`time_data_try_tai_minus_utc_mjd` returns `Some(offset)` for all query dates,
including dates before the table starts:

```rust
if mjd_utc < DayQuantity::new(first.start_mjd as f64) {
    return Some(utc_offset_seconds_in_segment(mjd_utc, first));  // silent extrapolation
}
```

This means `Time::<UTC>::try_from_chrono(date_in_1950)` succeeds and returns
a TAI-UTC offset extrapolated from the 1960 segment formula. UTC did not exist
in 1950; the extrapolated offset is not meaningful as standard UTC.

The test `pre_1961_utc_roundtrips_with_approximate_segment_extension`
acknowledges the approximation in its name but tests only round-trip closure,
not physical correctness.

## Current behavior
- `Time::<UTC>::try_from_chrono` succeeds for any date including pre-1960.
- The TAI-UTC offset used is the first segment's formula extended backward.
- The round-trip (UTC→TAI→UTC) is internally consistent but physically dubious.
- `ConversionError::UtcHistoryUnsupported` is never returned from this path.

## How it is currently handled
The behavior is intentional. The test confirms it is an expected approximation.
No documentation explains this to API callers.

## Pros of the current handling
- Allows roundtrip-consistent handling of legacy timestamps that predate UTC
  definition, which some astronomical archives carry as nominal UTC values.
- Simple: no boundary check or error path needed.

## Cons of the current handling
- Silently converts pre-1960 dates as if UTC existed, returning offsets with
  no physical basis.
- Users who query TAI-UTC for e.g. 1950 receive a concrete number without
  any indication that it is an approximation or outside the table's domain.
- `UtcHistoryUnsupported` variant exists but is unused, giving a false
  impression that the library detects and surfaces this case.

## Evidence
- `tempoch-core/src/data/active.rs`, lines 220–233
  (`time_data_try_tai_minus_utc_mjd`)
- `tempoch-core/src/data/active.rs`, lines 680–692
  (`pre_1961_utc_roundtrips_with_approximate_segment_extension`)
- `tempoch_time_data`, `UtcHistoryUnsupported` error variant

## User impact
- Users who build on `Time::<UTC>` for historical astronomy (pre-1960)
  receive offsets without any signal that the values are extrapolated.
- This is a documentation and API-clarity gap; the algorithmic behavior is
  self-consistent.

## What could be done to solve or reduce it
1. **Document the behavior** in `from_unix_seconds`, `try_from_chrono`, and
   `time_data_try_tai_minus_utc_mjd`: state explicitly that pre-1960 UTC is
   modeled by extending the first table segment and is not historically defined.
2. **Add a span constant** `UTC_HISTORY_START_MJD` so callers can guard the
   boundary themselves.
3. **Return `None`** from `time_data_try_tai_minus_utc_mjd` for pre-history
   dates and propagate the error as `UtcHistoryUnsupported`. This is the strict
   option but breaks backward compatibility.
4. **Introduce a feature flag or opt-in** for pre-history extrapolation, keeping
   the default strict.

## What cannot be solved without tradeoffs
- Strict rejection of pre-1960 UTC would break applications that carry nominal
  pre-UTC timestamps in legacy data. Some users actively need the extrapolation.
- Providing a physically accurate pre-1960 time scale would require integrating
  a separate UT2/universal-time model for the rubber-second era (1958–1972),
  which is a significant scope increase.

## Acceptance criteria for closing
- The API documentation for all UTC entry points states clearly when the
  TAI-UTC offset is extrapolated and what the boundary is.
- Either the `UtcHistoryUnsupported` variant is used and documented, or it is
  removed and replaced with documented extrapolation behavior.
