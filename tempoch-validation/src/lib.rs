// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! tempoch-validation — cross-validation harness.
//!
//! This crate is **dev/test-only** (`publish = false`). It holds:
//!
//! * Golden vectors from CSPICE/NAIF (ET/UTC) — committed CSVs under
//!   `data/spice/`. Regeneration requires CSPICE and is gated behind the
//!   `regenerate` feature; normal `cargo test` consumes the checked-in CSVs.
//! * Golden vectors from SOFA/ERFA (UTC/TAI/TT/UT1) — committed CSVs under
//!   `data/sofa/`.
//! * GNSS ICD reference points (epochs, week-rollover, seconds-of-week edges) —
//!   committed under `data/gnss/`.
//! * IERS/USNO EOP and ΔT reference samples (largely covered by tempoch-core's
//!   bundled tables; this crate adds boundary tests).
//!
//! See `tests/` for the actual test entry points.

/// Tolerance budgets, documented per conversion class.
pub mod tolerance {
    /// Two continuous SI-second scales (e.g. TAI↔TT): exact integer offset.
    pub const CONTINUOUS_OFFSET_NS: i128 = 1; // 1 ns
    /// TT↔TDB via Fairhead-Bretagnon: ~10 µs over 1600-2200 TT.
    pub const TT_TDB_NS: i128 = 10_000;
    /// UTC↔TAI: exact at integer-leap boundaries; allow 1 ns numerical noise.
    pub const UTC_TAI_NS: i128 = 1;
    /// UT1↔TT via bundled monthly ΔT: documented at <15 ms over observed
    /// overlap.
    pub const UT1_TT_MS: f64 = 15.0;
    /// GNSS system-time integer offsets vs TAI: exact.
    pub const GNSS_TAI_NS: i128 = 1;
}
