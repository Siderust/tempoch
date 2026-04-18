// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon
//
// Refresh generated tempoch-core time-data tables from official sources.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use sha2::{Digest, Sha256};
use tempoch_time_data_updater::{
    build_modern_delta_t_points, parse_delta_t_observed, parse_delta_t_predictions,
    parse_utc_tai_segments, render_generated_module, Provenance, Sources, DELTA_T_OBSERVED_URL,
    DELTA_T_PREDICTIONS_URL, PRE_1961_TAI_MINUS_UTC_APPROX, UTC_TAI_HISTORY_URL,
};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is .../tempoch-time-data-updater; root is its parent.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("crate has a workspace parent")
        .to_path_buf()
}

fn output_path() -> PathBuf {
    workspace_root()
        .join("tempoch-core")
        .join("src")
        .join("generated")
        .join("time_data.rs")
}

fn provenance_sidecar_path() -> PathBuf {
    workspace_root()
        .join("tempoch-core")
        .join("src")
        .join("generated")
        .join("time_data.provenance.json")
}

fn fetch_text(url: &str) -> Result<(String, String), String> {
    let response = ureq::get(url)
        .set("User-Agent", "tempoch-time-data-updater/1.0")
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|e| format!("fetch {url} failed: {e}"))?;
    let bytes = {
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut response.into_reader(), &mut buf)
            .map_err(|e| format!("read {url} body failed: {e}"))?;
        buf
    };
    let sha = {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hex(&hasher.finalize())
    };
    let text = String::from_utf8(bytes).map_err(|e| format!("{url} is not UTF-8: {e}"))?;
    Ok((text, sha))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
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

fn write_provenance(
    path: &Path,
    fetched_utc: &str,
    provenance: &Provenance<'_>,
) -> std::io::Result<()> {
    let data = serde_json::json!({
        "fetched_utc": fetched_utc,
        "utc_tai_sha256": provenance.utc_tai_sha,
        "delta_t_observed_sha256": provenance.delta_t_observed_sha,
        "delta_t_predictions_sha256": provenance.delta_t_predictions_sha,
    });
    let mut s = serde_json::to_string_pretty(&data).expect("serde_json::to_string_pretty");
    s.push('\n');
    std::fs::write(path, s)
}

fn run(check_only: bool) -> Result<i32, String> {
    let fetch_ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let (utc_tai_history, utc_tai_sha) = fetch_text(UTC_TAI_HISTORY_URL)?;
    let (delta_t_observed, delta_t_obs_sha) = fetch_text(DELTA_T_OBSERVED_URL)?;
    let (delta_t_predictions, delta_t_pred_sha) = fetch_text(DELTA_T_PREDICTIONS_URL)?;

    let utc_tai_segments = parse_utc_tai_segments(&utc_tai_history)?;
    let observed = parse_delta_t_observed(&delta_t_observed)?;
    let predicted = parse_delta_t_predictions(&delta_t_predictions)?;
    let (modern_delta_t_points, observed_end_mjd) =
        build_modern_delta_t_points(&observed, &predicted)?;

    let sources = Sources {
        utc_tai_history_url: UTC_TAI_HISTORY_URL,
        delta_t_observed_url: DELTA_T_OBSERVED_URL,
        delta_t_predictions_url: DELTA_T_PREDICTIONS_URL,
        pre_1961_tai_minus_utc_approx: PRE_1961_TAI_MINUS_UTC_APPROX,
    };
    let provenance = Provenance {
        utc_tai_sha: &utc_tai_sha,
        delta_t_observed_sha: &delta_t_obs_sha,
        delta_t_predictions_sha: &delta_t_pred_sha,
    };

    let rendered = render_generated_module(
        &utc_tai_segments,
        &modern_delta_t_points,
        observed_end_mjd,
        &sources,
        &provenance,
    );

    let out = output_path();

    if check_only {
        let current = std::fs::read_to_string(&out).unwrap_or_default();
        if current != rendered {
            eprintln!("{} is out of date", out.display());
            return Ok(1);
        }
        println!("{} is up to date", out.display());
        return Ok(0);
    }

    let changed = write_if_changed(&out, &rendered).map_err(|e| e.to_string())?;
    write_provenance(&provenance_sidecar_path(), &fetch_ts, &provenance)
        .map_err(|e| e.to_string())?;
    let status = if changed {
        "updated"
    } else {
        "already current"
    };
    println!(
        "{} {} (UTC-TAI segments={}, modern Delta T points={}, observed through MJD {:.0})",
        out.display(),
        status,
        utc_tai_segments.len(),
        modern_delta_t_points.len(),
        observed_end_mjd
    );
    Ok(0)
}

fn print_usage() {
    eprintln!(
        "Usage: tempoch-time-data-updater [--check]\n\
         \n\
         Regenerate tempoch-core/src/generated/time_data.rs from upstream sources.\n\
         With --check, exit non-zero if the committed file is out of date."
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
