// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon
//
// Refresh generated tempoch-core time-data tables from official sources.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use tempoch_core::runtime_data::{
    TimeDataManager, DELTA_T_OBSERVED_URL, DELTA_T_PREDICTIONS_URL, EOP_FINALS_URL,
    PRE_1961_TAI_MINUS_UTC_APPROX, UTC_TAI_HISTORY_URL,
};
use tempoch_time_data_updater::{render_eop_module, render_generated_module, Provenance, Sources};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is .../tempoch-time-data-updater; root is its parent.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("crate has a workspace parent")
        .to_path_buf()
}

fn generated_dir() -> PathBuf {
    workspace_root()
        .join("tempoch-core")
        .join("src")
        .join("generated")
}

fn time_data_path() -> PathBuf {
    generated_dir().join("time_data.rs")
}

fn eop_data_path() -> PathBuf {
    generated_dir().join("eop_data.rs")
}

fn provenance_sidecar_path() -> PathBuf {
    generated_dir().join("time_data.provenance.json")
}

fn write_if_changed(path: &Path, contents: &str) -> std::io::Result<bool> {
    if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        if existing == contents {
            return Ok(false);
        }
    }
    std::fs::write(path, contents)?;
    Ok(true)
}

fn provenance_json(fetched_utc: &str, provenance: &Provenance<'_>) -> String {
    let data = serde_json::json!({
        "fetched_utc": fetched_utc,
        "utc_tai_sha256": provenance.utc_tai_sha,
        "delta_t_observed_sha256": provenance.delta_t_observed_sha,
        "delta_t_predictions_sha256": provenance.delta_t_predictions_sha,
        "eop_finals_sha256": provenance.eop_finals_sha,
    });
    let mut s = serde_json::to_string_pretty(&data).expect("serde_json::to_string_pretty");
    s.push('\n');
    s
}

fn provenance_hashes_match(data: &serde_json::Value, provenance: &Provenance<'_>) -> bool {
    data.get("utc_tai_sha256")
        .and_then(serde_json::Value::as_str)
        == Some(provenance.utc_tai_sha)
        && data
            .get("delta_t_observed_sha256")
            .and_then(serde_json::Value::as_str)
            == Some(provenance.delta_t_observed_sha)
        && data
            .get("delta_t_predictions_sha256")
            .and_then(serde_json::Value::as_str)
            == Some(provenance.delta_t_predictions_sha)
        && data
            .get("eop_finals_sha256")
            .and_then(serde_json::Value::as_str)
            == Some(provenance.eop_finals_sha)
}

fn write_provenance(
    path: &Path,
    fetched_utc: &str,
    provenance: &Provenance<'_>,
) -> std::io::Result<bool> {
    let preserved_timestamp = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter(|json| provenance_hashes_match(json, provenance))
        .and_then(|json| {
            json.get("fetched_utc")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    let rendered = provenance_json(
        preserved_timestamp.as_deref().unwrap_or(fetched_utc),
        provenance,
    );
    write_if_changed(path, &rendered)
}

fn provenance_is_current(path: &Path, provenance: &Provenance<'_>) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    provenance_hashes_match(&json, provenance)
        && json
            .get("fetched_utc")
            .and_then(serde_json::Value::as_str)
            .is_some()
}

fn run(check_only: bool) -> Result<i32, String> {
    let temp_dir = std::env::temp_dir().join(format!(
        "tempoch_time_data_updater_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let manager = TimeDataManager::with_dir(&temp_dir).map_err(|e| e.to_string())?;
    let data = manager.refresh_and_load().map_err(|e| e.to_string())?;

    let sources = Sources {
        utc_tai_history_url: UTC_TAI_HISTORY_URL,
        delta_t_observed_url: DELTA_T_OBSERVED_URL,
        delta_t_predictions_url: DELTA_T_PREDICTIONS_URL,
        eop_finals_url: EOP_FINALS_URL,
        pre_1961_tai_minus_utc_approx: PRE_1961_TAI_MINUS_UTC_APPROX,
    };
    let provenance = Provenance {
        utc_tai_sha: data.provenance().utc_tai_sha256(),
        delta_t_observed_sha: data.provenance().delta_t_observed_sha256(),
        delta_t_predictions_sha: data.provenance().delta_t_predictions_sha256(),
        eop_finals_sha: data.provenance().eop_finals_sha256(),
    };

    let rendered_time = render_generated_module(
        data.utc_tai_segments(),
        data.modern_delta_t_points(),
        data.modern_delta_t_observed_end_mjd().value(),
        &sources,
        &provenance,
    );
    let rendered_eop = render_eop_module(data.eop_points(), &sources, &provenance);

    let time_out = time_data_path();
    let eop_out = eop_data_path();

    if check_only {
        let mut stale = false;
        for (path, rendered) in [(&time_out, &rendered_time), (&eop_out, &rendered_eop)] {
            let current = std::fs::read_to_string(path).unwrap_or_default();
            if &current != rendered {
                eprintln!("{} is out of date", path.display());
                stale = true;
            } else {
                println!("{} is up to date", path.display());
            }
        }
        let provenance_path = provenance_sidecar_path();
        if provenance_is_current(&provenance_path, &provenance) {
            println!("{} is up to date", provenance_path.display());
        } else {
            eprintln!("{} is out of date", provenance_path.display());
            stale = true;
        }
        return Ok(if stale { 1 } else { 0 });
    }

    let time_changed = write_if_changed(&time_out, &rendered_time).map_err(|e| e.to_string())?;
    let eop_changed = write_if_changed(&eop_out, &rendered_eop).map_err(|e| e.to_string())?;
    let provenance_changed = write_provenance(
        &provenance_sidecar_path(),
        data.provenance().fetched_utc(),
        &provenance,
    )
    .map_err(|e| e.to_string())?;
    let tag = |changed: bool| {
        if changed {
            "updated"
        } else {
            "already current"
        }
    };
    println!(
        "{} {} (UTC-TAI segments={}, modern Delta T points={}, observed through MJD {:.0})",
        time_out.display(),
        tag(time_changed),
        data.utc_tai_segments().len(),
        data.modern_delta_t_points().len(),
        data.modern_delta_t_observed_end_mjd().value()
    );
    println!(
        "{} {} (EOP points={}, observed through MJD {}, last MJD {})",
        eop_out.display(),
        tag(eop_changed),
        data.eop_points().len(),
        data.eop_observed_end_mjd().value() as i32,
        data.eop_end_mjd().value() as i32,
    );
    println!(
        "{} {}",
        provenance_sidecar_path().display(),
        tag(provenance_changed),
    );
    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(0)
}

fn print_usage() {
    eprintln!(
        "Usage: tempoch-time-data-updater [--check]\n\
         \n\
         Regenerate tempoch-core/src/generated/{{time_data,eop_data}}.rs and the\n\
         provenance sidecar from upstream sources.\n\
         With --check, exit non-zero if any committed generated file is out of date."
    );
}

fn main() -> ExitCode {
    let mut check = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--check" => check = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_usage();
                return ExitCode::from(2);
            }
        }
    }
    match run(check) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
