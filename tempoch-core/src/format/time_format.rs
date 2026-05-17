// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Scale-orthogonal [`TimeFormat`] trait.
//!
//! Built-in markers live in [`crate::format::markers`].

use core::fmt;

use crate::foundation::sealed::Sealed;
use qtty::Unit;

/// Marker trait for an external time encoding such as JD or Unix time.
#[allow(private_bounds)]
pub trait TimeFormat: Sealed + Copy + Clone + fmt::Debug + 'static {
    type Unit: Unit;
    const NAME: &'static str;
}

#[allow(unused_imports)]
pub use crate::format::markers::{J2000s, Unix, GPS, JD, MJD};
