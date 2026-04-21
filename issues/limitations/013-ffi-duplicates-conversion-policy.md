# The FFI layer duplicates conversion policy instead of adapting a single core model

## Summary
`tempoch-ffi` rebuilds a substantial conversion matrix and default-policy layer
in its own code instead of delegating through a single higher-level core
abstraction.

## Status
Pending architectural cleanup. The current FFI works, but it creates a second
policy surface that must stay manually aligned with the Rust model.

## What is the issue
The Rust API now has a clear conceptual model:

- typed `Time<S>` values
- explicit conversion targets
- `TimeContext` for context-required routes

The FFI does not simply expose that model in scalar form. Instead it contains
its own scale-dispatch helpers, its own TT/JD bridging logic, and its own
default-context policy decisions. That creates duplicate architecture across
two crates.

## Current behavior
- `tempoch-ffi` has its own `TempochScaleId` dispatch matrix.
- It converts through local helpers such as `scale_value_to_tt`,
  `tt_to_scale_value`, `jd_to_scale_value`, and `scale_value_to_jd`.
- UT1 routes implicitly use `TimeContext::new()` inside FFI helper code.
- Rust and FFI behavior must stay aligned by convention and tests.

## How it is currently handled
- The scalar C ABI is intentionally small and regular.
- FFI code reimplements the dispatch layer directly on top of `tempoch`.
- Tests cover representative roundtrips and error cases to catch drift.

## Pros of the current handling
- Produces a compact and language-agnostic C ABI.
- Keeps adapter code simple on the C side.
- Lets the FFI optimize for scalar values without exposing Rust generic shapes.

## Cons of the current handling
- Policy and conversion semantics now live in two places.
- New scale behavior or context policy changes require synchronized updates.
- The FFI cannot expose richer context/data selection without another parallel
  design pass.
- Drift risk grows as the Rust architecture evolves.

## Evidence
- `tempoch-ffi/src/carriers.rs`
- `tempoch-ffi/src/time.rs`
- `tempoch-core/src/target.rs`
- `tempoch-core/src/scale/conversion.rs`

## User impact
- Adapter maintainers depend on the FFI staying manually aligned with the Rust
  crate's semantics.
- New architectural features in Rust can lag or arrive in reduced form at the
  C ABI boundary.
- UT1 and runtime-data policy choices are less explicit in FFI than in Rust.

## What could be done to solve or reduce it
- Introduce a narrower core adapter layer specifically meant for scalar/FFI
  dispatch so the matrix lives in one place.
- Expose explicit context or data-source handles in the FFI if UT1/runtime-data
  policy is meant to be controllable there.
- Reduce duplicated helper logic by routing more of the FFI through one stable
  conversion service inside `tempoch-core`.

## What cannot be solved without tradeoffs
- A compact scalar C ABI will never mirror Rust generics one-to-one.
- Some translation layer is unavoidable, but duplication of policy and
  conversion wiring is not.

## Acceptance criteria for closing
- Either centralize the conversion/policy matrix behind a single core adapter
  layer, or explicitly freeze the FFI as a separately maintained policy
  surface with its own documented constraints.
