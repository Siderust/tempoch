// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

#[cfg(test)]
use chrono::{DateTime, Utc};
use siderust_archive::time::TimeDataBundle;
#[cfg(any(test, feature = "runtime-data-fetch"))]
use siderust_archive::time::TimeDataError as InternalDataError;
#[cfg(feature = "runtime-data-fetch")]
use siderust_archive::time::TimeDataManager;
#[cfg(test)]
use std::sync::Mutex;
use std::sync::{Arc, OnceLock, RwLock};

#[cfg(test)]
const RUNTIME_DATA_MAX_AGE_SECONDS: i64 = 24 * 60 * 60;

static COMPILED_TIME_DATA: OnceLock<Arc<TimeDataBundle>> = OnceLock::new();
static ACTIVE_TIME_DATA: OnceLock<RwLock<Arc<TimeDataBundle>>> = OnceLock::new();

#[cfg(test)]
static TEST_TIME_DATA_GUARD: Mutex<()> = Mutex::new(());
#[cfg(test)]
static TEST_TIME_DATA: Mutex<Option<Arc<TimeDataBundle>>> = Mutex::new(None);

fn active_time_data_slot() -> &'static RwLock<Arc<TimeDataBundle>> {
    ACTIVE_TIME_DATA.get_or_init(|| RwLock::new(compiled_time_data()))
}

#[cfg(any(test, feature = "runtime-data-fetch"))]
pub(crate) fn set_active_time_data(bundle: TimeDataBundle) {
    let mut slot = active_time_data_slot()
        .write()
        .unwrap_or_else(|err| err.into_inner());
    *slot = Arc::new(bundle);
}

pub(crate) fn active_time_data() -> Arc<TimeDataBundle> {
    #[cfg(test)]
    if let Some(bundle) = TEST_TIME_DATA
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .clone()
    {
        return bundle;
    }

    active_time_data_slot()
        .read()
        .unwrap_or_else(|err| err.into_inner())
        .clone()
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
    let _guard = TEST_TIME_DATA_GUARD
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    let mut slot = TEST_TIME_DATA.lock().unwrap_or_else(|err| err.into_inner());
    let previous = slot.replace(Arc::new(data));
    drop(slot);
    let result = f();
    *TEST_TIME_DATA.lock().unwrap_or_else(|err| err.into_inner()) = previous;
    result
}

#[cfg(test)]
pub(crate) fn with_runtime_data_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = TEST_TIME_DATA_GUARD
        .lock()
        .unwrap_or_else(|err| err.into_inner());
    f()
}

pub(crate) fn compiled_time_data() -> Arc<TimeDataBundle> {
    COMPILED_TIME_DATA
        .get_or_init(|| Arc::new(siderust_archive::time::bundled_time_data()))
        .clone()
}
