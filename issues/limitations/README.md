# `tempoch` Pending Limitations

This directory now tracks only the `tempoch` limitations that still remain
open as scientific or API boundaries.

Resolved items were removed from this directory after implementation:

- `001` UTC raw-axis access and second-based arithmetic
- `002` pre-1961 UTC hard failure
- `004` missing default UT1 route
- `005` non-automatic `runtime-data` activation
- `010` lack of a scale-tagged serde wire format
- `011` `TimeContext` now snapshots the active time-data bundle
- `012` runtime data support is no longer feature-gated
- `013` FFI layer duplicates conversion policy (centralized in `tempoch-core::scalar`)
- `014` public crates depend on a non-published internal crate (`TimeDataError` moved to `tempoch-core::error`)

Still pending:

- [TT↔TDB uses a bounded approximation window](./003-tt-tdb-approximation-window.md)
- [ΔT extrapolation beyond the horizon is scientifically unsupported](./006-delta-t-extrapolation-unsupported.md)
- [Built-in EOP coverage is finite and optional fields may be absent](./007-eop-coverage-and-missing-fields.md)
- [Bundled UT1 accuracy claims are date-qualified, not timeless](./008-ut1-bundle-qualified-accuracy.md)
- [The scale set is sealed](./009-scale-set-is-sealed.md)
- [TDB-TT accuracy is documented inconsistently across files](./015-tdb-tt-accuracy-docs-inconsistent.md)
- [TCB↔TDB conversions drop the compensated pair](./016-tcb-tdb-drops-compensated-pair.md)
- [UTC pre-history is silently extrapolated rather than documented or bounded](./017-utc-pre-history-silent-extrapolation.md)
- [ScaleKind::Gps encodes days, not conventional GPS seconds](./018-scale-kind-gps-uses-days-not-seconds.md)
