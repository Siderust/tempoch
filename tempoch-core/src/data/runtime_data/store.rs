// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

use crate::archive::time::TimeDataBundle;
#[cfg(any(test, feature = "runtime-data-fetch"))]
use crate::archive::time::TimeDataError as InternalDataError;
#[cfg(feature = "runtime-data-fetch")]
use crate::archive::time::TimeDataManager;
use crate::data::status::ActiveTimeDataSource;
use alloc::sync::Arc;

#[cfg(test)]
use chrono::{DateTime, Utc};
#[cfg(feature = "std")]
use std::sync::{OnceLock, RwLock};

#[cfg(test)]
const RUNTIME_DATA_MAX_AGE_SECONDS: i64 = 24 * 60 * 60;

#[cfg(feature = "std")]
static COMPILED_TIME_DATA: OnceLock<Arc<TimeDataBundle>> = OnceLock::new();

#[cfg(feature = "std")]
static ACTIVE_TIME_DATA: OnceLock<RwLock<Arc<TimeDataBundle>>> = OnceLock::new();

#[cfg(test)]
mod test_overrides {
    use super::*;
    use std::{cell::RefCell, thread::LocalKey};

    std::thread_local! {
        pub(super) static OVERRIDE: RefCell<Option<Arc<TimeDataBundle>>> = const { RefCell::new(None) };
        pub(super) static ACTIVE: RefCell<Option<Arc<TimeDataBundle>>> = const { RefCell::new(None) };
    }

    pub(super) fn scoped<T>(
        slot: &'static LocalKey<RefCell<Option<Arc<TimeDataBundle>>>>,
        replacement: Option<Arc<TimeDataBundle>>,
        f: impl FnOnce() -> T,
    ) -> T {
        struct Restore {
            slot: &'static LocalKey<RefCell<Option<Arc<TimeDataBundle>>>>,
            previous: Option<Arc<TimeDataBundle>>,
        }
        impl Drop for Restore {
            fn drop(&mut self) {
                self.slot
                    .with(|slot| *slot.borrow_mut() = self.previous.take());
            }
        }
        let _restore = Restore {
            slot,
            previous: slot.with(|slot| slot.replace(replacement)),
        };
        f()
    }
}

#[cfg(feature = "std")]
fn active_time_data_slot() -> &'static RwLock<Arc<TimeDataBundle>> {
    ACTIVE_TIME_DATA.get_or_init(|| RwLock::new(compiled_time_data()))
}

#[cfg(any(test, feature = "runtime-data-fetch"))]
pub(crate) fn set_active_time_data(bundle: TimeDataBundle) {
    #[cfg(test)]
    test_overrides::ACTIVE.with(|slot| *slot.borrow_mut() = Some(Arc::new(bundle)));
    #[cfg(not(test))]
    {
        let mut slot = active_time_data_slot()
            .write()
            .unwrap_or_else(|err| err.into_inner());
        *slot = Arc::new(bundle);
    }
}

pub(crate) fn active_time_data() -> Arc<TimeDataBundle> {
    #[cfg(test)]
    if let Some(bundle) = test_overrides::OVERRIDE
        .with(|slot| slot.borrow().clone())
        .or_else(|| test_overrides::ACTIVE.with(|slot| slot.borrow().clone()))
    {
        return bundle;
    }

    #[cfg(feature = "std")]
    {
        active_time_data_slot()
            .read()
            .unwrap_or_else(|err| err.into_inner())
            .clone()
    }

    #[cfg(not(feature = "std"))]
    compiled_time_data()
}

pub(crate) fn active_time_data_source() -> ActiveTimeDataSource {
    #[cfg(test)]
    if test_overrides::OVERRIDE.with(|slot| slot.borrow().is_some()) {
        return ActiveTimeDataSource::Override;
    }

    let bundle = active_time_data();
    if bundle.provenance().fetched_utc() == "compiled" {
        ActiveTimeDataSource::Bundled
    } else {
        ActiveTimeDataSource::RuntimeCache
    }
}

/// Load runtime time data into the active bundle.
///
/// This is cache-first: it uses the current cached bundle if present,
/// falling back to a refresh when no valid cache is available.
#[cfg(feature = "runtime-data-fetch")]
pub fn update_runtime_time_data() -> Result<(), crate::foundation::error::TimeDataError> {
    load_and_activate_runtime_time_data(false).map_err(Into::into)
}

/// Force-refresh runtime time data and load it into the active bundle.
#[cfg(feature = "runtime-data-fetch")]
pub fn refresh_runtime_time_data() -> Result<(), crate::foundation::error::TimeDataError> {
    load_and_activate_runtime_time_data(true).map_err(Into::into)
}

/// Explicitly fetch the latest runtime time data and load it into the active
/// bundle.
#[cfg(feature = "runtime-data-fetch")]
pub fn fetch_latest_time_data() -> Result<(), crate::foundation::error::TimeDataError> {
    refresh_runtime_time_data()
}

#[cfg(feature = "runtime-data-fetch")]
fn load_and_activate_runtime_time_data(force_refresh: bool) -> Result<(), InternalDataError> {
    let manager = TimeDataManager::new()?;
    let bundle = select_time_data(
        manager.load_cached(),
        || manager.refresh_and_load(),
        force_refresh,
    )?;
    set_active_time_data(bundle);
    Ok(())
}

#[cfg(any(test, feature = "runtime-data-fetch"))]
pub(crate) fn select_time_data(
    cached: Result<TimeDataBundle, InternalDataError>,
    refresh: impl FnOnce() -> Result<TimeDataBundle, InternalDataError>,
    force_refresh: bool,
) -> Result<TimeDataBundle, InternalDataError> {
    if force_refresh {
        return refresh();
    }

    match cached {
        Ok(bundle) => Ok(bundle),
        Err(_) => refresh(),
    }
}

#[cfg(test)]
fn bundle_is_stale(bundle: &TimeDataBundle, now: DateTime<Utc>) -> bool {
    match bundle.provenance().fetched_at() {
        Some(fetched_at) => {
            now.signed_duration_since(fetched_at).num_seconds() > RUNTIME_DATA_MAX_AGE_SECONDS
        }
        None => true,
    }
}

#[cfg(test)]
pub(crate) fn select_time_data_for_auto_refresh(
    cached: Result<TimeDataBundle, InternalDataError>,
    refresh: impl FnOnce() -> Result<TimeDataBundle, InternalDataError>,
    now: DateTime<Utc>,
) -> Result<TimeDataBundle, InternalDataError> {
    match cached {
        Ok(bundle) if !bundle_is_stale(&bundle, now) => Ok(bundle),
        Ok(bundle) => refresh().or(Ok(bundle)),
        Err(_) => refresh(),
    }
}

#[cfg(test)]
pub(crate) fn with_test_time_data<T>(data: TimeDataBundle, f: impl FnOnce() -> T) -> T {
    test_overrides::scoped(&test_overrides::OVERRIDE, Some(Arc::new(data)), f)
}

#[cfg(test)]
pub(crate) fn with_runtime_data_lock<T>(f: impl FnOnce() -> T) -> T {
    let previous = test_overrides::ACTIVE.with(|slot| slot.borrow().clone());
    test_overrides::scoped(&test_overrides::ACTIVE, previous, f)
}

pub(crate) fn compiled_time_data() -> Arc<TimeDataBundle> {
    #[cfg(feature = "std")]
    {
        COMPILED_TIME_DATA
            .get_or_init(|| Arc::new(crate::archive::time::bundled_time_data()))
            .clone()
    }
    #[cfg(not(feature = "std"))]
    Arc::new(crate::archive::time::bundled_time_data())
}

#[cfg(test)]
mod isolation_tests {
    use super::*;

    #[test]
    fn synthetic_bundle_does_not_leak_to_other_test_threads() {
        with_test_time_data((*compiled_time_data()).clone(), || {
            let here = active_time_data();
            let elsewhere = std::thread::spawn(active_time_data).join().unwrap();
            assert!(!Arc::ptr_eq(&here, &elsewhere));
        });
        assert!(Arc::ptr_eq(&active_time_data(), &compiled_time_data()));
    }

    #[test]
    fn nested_synthetic_overrides_restore_the_outer_bundle() {
        let before = active_time_data();
        let before_source = active_time_data_source();

        with_test_time_data((*compiled_time_data()).clone(), || {
            let outer = active_time_data();
            assert_eq!(active_time_data_source(), ActiveTimeDataSource::Override);

            with_test_time_data((*compiled_time_data()).clone(), || {
                let inner = active_time_data();
                assert!(!Arc::ptr_eq(&inner, &outer));
                assert_eq!(active_time_data_source(), ActiveTimeDataSource::Override);
            });

            assert!(Arc::ptr_eq(&active_time_data(), &outer));
        });

        assert!(Arc::ptr_eq(&active_time_data(), &before));
        assert_eq!(active_time_data_source(), before_source);
    }

    #[test]
    fn synthetic_overrides_restore_after_unwind() {
        let before = active_time_data();
        let result = std::panic::catch_unwind(|| {
            with_test_time_data((*compiled_time_data()).clone(), || {
                assert_eq!(active_time_data_source(), ActiveTimeDataSource::Override);
                panic!("deliberate scoped override unwind");
            });
        });
        assert!(result.is_err());
        assert!(Arc::ptr_eq(&active_time_data(), &before));
    }

    #[test]
    fn runtime_active_bundle_is_thread_local_and_restored_on_unwind() {
        let before = active_time_data();
        let other_thread_before = std::thread::spawn(active_time_data).join().unwrap();

        let result = std::panic::catch_unwind(|| {
            with_runtime_data_lock(|| {
                set_active_time_data((*compiled_time_data()).clone());
                let active = active_time_data();
                assert!(!Arc::ptr_eq(&active, &before));

                let other_thread = std::thread::spawn(active_time_data).join().unwrap();
                assert!(Arc::ptr_eq(&other_thread, &other_thread_before));

                with_runtime_data_lock(|| {
                    set_active_time_data((*compiled_time_data()).clone());
                    assert!(!Arc::ptr_eq(&active_time_data(), &active));
                });
                assert!(Arc::ptr_eq(&active_time_data(), &active));

                panic!("deliberate active bundle unwind");
            });
        });
        assert!(result.is_err());
        assert!(Arc::ptr_eq(&active_time_data(), &before));
    }
}
