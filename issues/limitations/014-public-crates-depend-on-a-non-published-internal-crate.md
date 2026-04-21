# Public crates depend on a non-published internal support crate

## Summary
The public `tempoch` crate family depends on `tempoch-time-data`, but that
support crate is marked `publish = false`.

## Status
Pending architectural cleanup. This may be manageable inside the current
workspace release flow, but it is an awkward package boundary for a public
crate family.

## What is the issue
`tempoch-time-data` is structurally important:

- `tempoch-core` depends on it directly
- its types are part of the runtime-data story
- `TimeDataError` is reexported publicly

At the same time, the crate declares `publish = false`. That creates an odd
boundary where published crates rely on a workspace-internal crate that is not
itself presented as a publishable public component.

## Current behavior
- `tempoch-time-data` is an internal workspace crate marked `publish = false`.
- `tempoch-core` depends on it directly.
- Public APIs reexport `TimeDataError` from that crate.

## How it is currently handled
- The workspace treats `tempoch-time-data` as an implementation detail.
- Release logic presumably relies on workspace publishing behavior rather than
  treating the crate as an independently consumable package.
- External users mostly see the top-level crates, not the internal split.

## Pros of the current handling
- Keeps the support crate clearly branded as internal.
- Gives maintainers freedom to refactor internals without advertising another
  first-class product.
- Avoids committing to separate user-facing docs and positioning for the data
  support crate.

## Cons of the current handling
- Weakens the package-layer architecture for public releases.
- Makes the semver and publishing story harder to reason about cleanly.
- Public crates end up exposing types from a crate that is not meant to stand
  on its own.
- Future tooling or publishing changes may surface this boundary as friction.

## Evidence
- `tempoch-core/Cargo.toml`
- `tempoch-time-data/Cargo.toml`
- `tempoch-core/src/lib.rs`

## User impact
- Most users will not notice immediately, but release engineering and package
  architecture become more fragile.
- Public API stability is partially tied to a crate that is not treated as a
  publishable public unit.

## What could be done to solve or reduce it
- Make `tempoch-time-data` a publishable support crate with a deliberately
  scoped API.
- Keep it internal but stop reexporting its types across public crate
  boundaries.
- Collapse the internal crate back into `tempoch-core` if it is not meant to
  be a real package boundary.
- Split pure public data-model pieces from private runtime-management code.

## What cannot be solved cleanly without choosing a boundary
- The current setup cannot be both a purely private internal crate and a fully
  public package boundary at the same time.
- If public crates continue to expose its types, the support crate is already
  part of the effective public architecture whether or not it is published.

## Acceptance criteria for closing
- Either make the support crate a deliberate publishable boundary, or fully
  internalize it so published crates no longer expose or depend on it as part
  of their public-facing architecture.
