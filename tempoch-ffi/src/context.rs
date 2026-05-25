// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

//! Opaque time-conversion contexts for the C ABI.

use crate::catch_panic;
use crate::error::TempochStatus;
use tempoch::TimeContext;

/// Opaque FFI time-conversion context.
pub struct TempochContext {
    pub(crate) inner: TimeContext,
}

/// Create a default context backed by the monthly ΔT model.
///
/// # Safety
/// `out` must be a valid, non-null pointer to writable storage for a context handle.
#[no_mangle]
pub unsafe extern "C" fn tempoch_context_create_default(
    out: *mut *mut TempochContext,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let ctx = Box::new(TempochContext {
            inner: TimeContext::new(),
        });
        // SAFETY: `out` was checked for null and the function safety contract
        // requires it to point to writable storage for one context handle.
        unsafe { *out = Box::into_raw(ctx) };
        TempochStatus::Ok
    })
}

/// Create a context that prefers the compiled builtin EOP path for UT1.
///
/// # Safety
/// `out` must be a valid, non-null pointer to writable storage for a context handle.
#[no_mangle]
pub unsafe extern "C" fn tempoch_context_create_with_builtin_eop(
    out: *mut *mut TempochContext,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if out.is_null() {
            return TempochStatus::NullPointer;
        }
        let ctx = Box::new(TempochContext {
            inner: TimeContext::with_builtin_eop(),
        });
        // SAFETY: `out` was checked for null and the function safety contract
        // requires it to point to writable storage for one context handle.
        unsafe { *out = Box::into_raw(ctx) };
        TempochStatus::Ok
    })
}

/// Derive a new context that permits pre-1961 UTC extrapolation.
///
/// # Safety
/// `handle` must be a valid, non-null context pointer produced by this crate, and `out` must be
/// a valid, non-null pointer to writable storage for a context handle.
#[no_mangle]
pub unsafe extern "C" fn tempoch_context_allow_pre_definition_utc(
    handle: *const TempochContext,
    out: *mut *mut TempochContext,
) -> TempochStatus {
    catch_panic!(TempochStatus::InternalPanic, {
        if handle.is_null() || out.is_null() {
            return TempochStatus::NullPointer;
        }
        let derived = unsafe { (*handle).inner.clone().allow_pre_definition_utc() };
        let ctx = Box::new(TempochContext { inner: derived });
        // SAFETY: `out` was checked for null and the function safety contract
        // requires it to point to writable storage for one context handle.
        unsafe { *out = Box::into_raw(ctx) };
        TempochStatus::Ok
    })
}

/// Free a context handle previously returned by `tempoch_context_create_*`.
///
/// # Safety
/// `handle` must be either null or a live pointer produced by this crate.
#[no_mangle]
pub unsafe extern "C" fn tempoch_context_free(handle: *mut TempochContext) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn create_contexts_and_free() {
        let mut handle: *mut TempochContext = ptr::null_mut();
        assert_eq!(
            unsafe { tempoch_context_create_default(&mut handle) },
            TempochStatus::Ok
        );
        assert!(!handle.is_null());
        unsafe { tempoch_context_free(handle) };

        let mut builtin: *mut TempochContext = ptr::null_mut();
        assert_eq!(
            unsafe { tempoch_context_create_with_builtin_eop(&mut builtin) },
            TempochStatus::Ok
        );
        assert!(!builtin.is_null());
        unsafe { tempoch_context_free(builtin) };
    }

    #[test]
    fn derive_pre_definition_context() {
        let mut handle: *mut TempochContext = ptr::null_mut();
        let mut derived: *mut TempochContext = ptr::null_mut();
        assert_eq!(
            unsafe { tempoch_context_create_default(&mut handle) },
            TempochStatus::Ok
        );
        assert_eq!(
            unsafe { tempoch_context_allow_pre_definition_utc(handle, &mut derived) },
            TempochStatus::Ok
        );
        assert!(!derived.is_null());
        unsafe {
            tempoch_context_free(handle);
            tempoch_context_free(derived);
        }
    }
}
