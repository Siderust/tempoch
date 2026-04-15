// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Private storage layer. Storage is not a public contract.

use super::axis::Axis;
use super::error::ConversionError;
use core::marker::PhantomData;
use qtty::time::Seconds;

/// SI seconds since J2000 TT (JD 2_451_545.0 TT). The scalar is interpreted
/// on axis `A`: on TT it is SI seconds on the TT axis, on TAI it is SI
/// seconds on the TAI axis (i.e. offset by +32.184 s), and so on. Axis
/// conversion crosses this boundary via the `convert` module.
///
/// For continuous axes, the "leap" flag is always `false`.
/// For UTC, the scalar records the corresponding *TAI* seconds since J2000
/// TT (so it remains continuous across leap seconds), and `leap` marks the
/// instant as a positive-leap label.
#[derive(Debug, Copy, Clone)]
pub(crate) struct Storage<A: Axis> {
    pub(crate) seconds: Seconds,
    pub(crate) leap: bool,
    pub(crate) _axis: PhantomData<A>,
}

impl<A: Axis> Storage<A> {
    #[inline]
    pub(crate) const fn new_unchecked(seconds: Seconds, leap: bool) -> Self {
        Self {
            seconds,
            leap,
            _axis: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn new(seconds: Seconds) -> Result<Self, ConversionError> {
        if seconds.is_finite() {
            Ok(Self::new_unchecked(seconds, false))
        } else {
            Err(ConversionError::NonFinite)
        }
    }
}

/// Axis witness that the axis has a continuous SI-second storage and
/// supports direct `qtty::Second` arithmetic. UTC is deliberately excluded
/// (see RFC §9).
///
/// Sealed — downstream cannot implement it.
pub trait ContinuousAxis: Axis + super::sealed::Sealed {}

macro_rules! continuous {
    ($($axis:ty),+ $(,)?) => {
        $(impl ContinuousAxis for $axis {})+
    };
}
continuous!(
    super::axis::TAI,
    super::axis::TT,
    super::axis::TDB,
    super::axis::TCG,
    super::axis::TCB,
    super::axis::UT1,
);

// UTC intentionally does not implement ContinuousAxis.
