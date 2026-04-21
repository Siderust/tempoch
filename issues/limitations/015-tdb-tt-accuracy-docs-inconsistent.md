# TDB-TT accuracy is documented inconsistently across files

## Summary
`constats.rs` documents the TT↔TDB truncated Fairhead-Bretagnon series as
staying "below about 2 microseconds", while `scale/mod.rs` and `conversion.rs`
state "about 10 microseconds". Both numbers are real but refer to different
reference comparators. This creates ambiguous accuracy expectations for users.

## Status
Resolved. `constats.rs` updated to clarify the two-level error budget and
align the documented accuracy with the ~10 µs end-to-end figure used elsewhere.

## What is the issue
The library uses the 7-term truncated Fairhead-Bretagnon series (USNO Circular
179, Eq. 2.27) for TT↔TDB. There are two published accuracy figures for this
family of approximations:

- **~2 µs**: Truncation error of the 7-term series relative to the full
  Fairhead-Bretagnon (1990) series. This is the number in `constats.rs`.
- **~10 µs**: Error of the full Fairhead-Bretagnon series itself relative to
  numerical integration (e.g. JPL DE ephemerides). This is in the other files.

End-to-end accuracy of the implemented series is therefore on the order of
**10–12 µs** relative to the best-available reference. Stating "2 µs" without
qualification implies a much tighter ceiling than the method can support.

## Current behavior
- `constats.rs`, line 34: "documented to stay below about 2 microseconds"
- `scale/mod.rs` and `conversion.rs`: "about 10 microseconds"

## How it is currently handled
The discrepancy is passive. Users reading the constants file get a more
optimistic picture than users reading the conversion module.

## Pros of the current handling
- The 2 µs figure is technically accurate for the series-vs-series comparison.
- Neither claim is wrong in isolation.

## Cons of the current handling
- Users of `constats.rs` (e.g. anyone checking range constants before
  conversion) see a claimed accuracy that is 5× better than what is delivered
  against real ephemeris data.
- The two-step composition (truncated series error + full-series error) is not
  explained anywhere.

## Evidence
- `tempoch-core/src/constats.rs`, lines 33–43 (range constants and doc comment)
- `tempoch-core/src/scale/conversion.rs`, `tdb_minus_tt_seconds` doc comment
- `tempoch-core/src/scale/mod.rs`, TDB section

## User impact
- Documentation mismatch. Users relying on the `constats.rs` "2 µs" figure to
  validate whether TDB is accurate enough for their application may
  overestimate the method's accuracy.

## What could be done to solve or reduce it
- Align all documentation on **~10 µs** as the end-to-end accuracy figure for
  the 7-term truncated series.
- Where the 2 µs figure adds value, accompany it with the qualifier "relative
  to the full Fairhead-Bretagnon series; ~10 µs relative to numerical
  integration."
- Reference USNO Circular 179 Table 2.2 for traceability.

## What cannot be solved without tradeoffs
- Using a more accurate series (full Fairhead-Bretagnon or direct numerical
  integration) would improve the ceiling at the cost of significantly more
  computation and a larger constant table.

## Acceptance criteria for closing
- All documentation that states a TDB-TT accuracy figure agrees on the
  end-to-end comparison baseline and quotes a consistent number (or two numbers
  that are each clearly attributed to a specific comparison).
