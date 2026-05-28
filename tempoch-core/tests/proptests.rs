// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Property tests for W1 ExactDuration and W3 scale-conversion invariants.
//!
//! These tests verify high-level invariants over generated inputs:
//!
//! * `ExactDuration` normalization, sign symmetry, ordering, and
//!   `floor/ceil/round` invariants.
//! * `Time<S> → Time<S2> → Time<S>` round-trip preservation for every
//!   continuous-scale pair (within documented tolerances).
//! * `Time + d - d == Time` (modulo ExactDuration precision).

use proptest::prelude::*;
use qtty::Second;
use tempoch_core::{ExactDuration, Time, BDT, GPST, GST, QZSST, TAI, TCG, TT};

const J2000_SECONDS_RANGE: i64 = 3_155_760_000; // ~100 yr around J2000

prop_compose! {
    fn arb_nanos()(n in any::<i64>()) -> i128 { n as i128 }
}

prop_compose! {
    fn arb_duration()(n in arb_nanos()) -> ExactDuration {
        ExactDuration::from_nanos(n)
    }
}

prop_compose! {
    fn arb_j2000_seconds()(s in (-J2000_SECONDS_RANGE)..J2000_SECONDS_RANGE) -> Second {
        Second::new(s as f64)
    }
}

prop_compose! {
    fn arb_tt()(s in arb_j2000_seconds()) -> Time<TT> {
        Time::<TT>::from_raw_j2000_seconds(s).unwrap()
    }
}

prop_compose! {
    fn arb_tai()(s in arb_j2000_seconds()) -> Time<TAI> {
        Time::<TAI>::from_raw_j2000_seconds(s).unwrap()
    }
}

proptest! {
    #[test]
    fn duration_neg_round_trip(d in arb_duration()) {
        if d != ExactDuration::MIN {
            prop_assert_eq!(-(-d), d);
        }
    }

    #[test]
    fn duration_add_sub_inverse(a in arb_duration(), b in arb_duration()) {
        if let (Ok(sum), Ok(_)) = (a.checked_add(b), b.checked_neg()) {
            if let Ok(back) = sum.checked_sub(b) {
                prop_assert_eq!(back, a);
            }
        }
    }

    #[test]
    fn duration_ordering_matches_nanos(a in arb_duration(), b in arb_duration()) {
        prop_assert_eq!(a.cmp(&b), a.as_nanos_i128().cmp(&b.as_nanos_i128()));
    }

    #[test]
    fn duration_floor_le_ceil(d in arb_duration(), q_nanos in 1_i64..1_000_000_000_000) {
        let q = ExactDuration::from_nanos(q_nanos as i128);
        let floor = d.floor_to(q);
        let ceil = d.ceil_to(q);
        prop_assert!(floor.as_nanos_i128() <= d.as_nanos_i128());
        prop_assert!(ceil.as_nanos_i128() >= d.as_nanos_i128());
        // Range bound: ceil - floor is 0 (already aligned) or q.
        let span = ceil.as_nanos_i128() - floor.as_nanos_i128();
        prop_assert!(span == 0 || span == q_nanos as i128);
    }

    #[test]
    fn duration_round_lies_between_floor_and_ceil(
        d in arb_duration(),
        q_nanos in 1_i64..1_000_000_000_000,
    ) {
        let q = ExactDuration::from_nanos(q_nanos as i128);
        let round = d.round_to(q);
        let floor = d.floor_to(q);
        let ceil = d.ceil_to(q);
        prop_assert!(round == floor || round == ceil);
    }

    #[test]
    fn time_add_then_sub_preserves_instant(
        t in arb_tt(),
        d_ns in -1_000_000_000_000_i64..1_000_000_000_000_i64,
    ) {
        let d = ExactDuration::from_nanos(d_ns as i128);
        let shifted = t.add_exact(d).sub_exact(d);
        let back = shifted.diff_exact(t).unwrap();
        // Allow 100 ns drift from split-f64 storage round-trip.
        prop_assert!(back.as_nanos_i128().abs() < 100,
            "add/sub round-trip drift > 100 ns: {} ns", back.as_nanos_i128());
    }

    #[test]
    fn tai_round_trip_to_gpst_within_tolerance(t in arb_tai()) {
        let gpst = t.to::<GPST>();
        let back = gpst.to::<TAI>();
        let d = t.diff_exact(back).unwrap();
        prop_assert!(d.as_nanos_i128().abs() <= 1);
    }

    #[test]
    fn tai_round_trip_to_bdt_within_tolerance(t in arb_tai()) {
        let bdt = t.to::<BDT>();
        let back = bdt.to::<TAI>();
        let d = t.diff_exact(back).unwrap();
        prop_assert!(d.as_nanos_i128().abs() <= 1);
    }

    #[test]
    fn cross_gnss_round_trip_within_tolerance(t in arb_tai()) {
        for converted in [
            t.to::<GPST>().to::<GST>().to::<QZSST>().to::<BDT>().to::<TAI>(),
            t.to::<BDT>().to::<GPST>().to::<TAI>(),
            t.to::<GST>().to::<QZSST>().to::<TAI>(),
        ] {
            let d = t.diff_exact(converted).unwrap();
            prop_assert!(d.as_nanos_i128().abs() <= 10,
                "cross-GNSS round trip drift > 10 ns: {} ns", d.as_nanos_i128());
        }
    }

    #[test]
    fn tt_tcg_round_trip_within_continuous_tolerance(t in arb_tt()) {
        let tcg = t.to::<TCG>();
        let back = tcg.to::<TT>();
        let d = t.diff_exact(back).unwrap();
        // TT↔TCG is a tiny linear scaling; for a 100-yr range expect << 1 µs drift.
        prop_assert!(d.as_nanos_i128().abs() < 1_000,
            "TT↔TCG round trip drift > 1 µs: {} ns", d.as_nanos_i128());
    }
}
