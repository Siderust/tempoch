// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon
//
// Refresh generated tempoch-core time-data tables from official sources.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use tempoch_time_data::{
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
        .join("tempoch-time-data")
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

#[derive(Debug, Clone)]
struct OutputPaths {
    time_data: PathBuf,
    eop_data: PathBuf,
    provenance_sidecar: PathBuf,
}

fn default_output_paths() -> OutputPaths {
    OutputPaths {
        time_data: time_data_path(),
        eop_data: eop_data_path(),
        provenance_sidecar: provenance_sidecar_path(),
    }
}

fn bundled_sources() -> Sources<'static> {
    Sources {
        utc_tai_history_url: UTC_TAI_HISTORY_URL,
        delta_t_observed_url: DELTA_T_OBSERVED_URL,
        delta_t_predictions_url: DELTA_T_PREDICTIONS_URL,
        eop_finals_url: EOP_FINALS_URL,
        pre_1961_tai_minus_utc_approx: PRE_1961_TAI_MINUS_UTC_APPROX,
    }
}

fn bundle_provenance<'a>(data: &'a tempoch_time_data::TimeDataBundle) -> Provenance<'a> {
    Provenance {
        utc_tai_sha: data.provenance().utc_tai_sha256(),
        delta_t_observed_sha: data.provenance().delta_t_observed_sha256(),
        delta_t_predictions_sha: data.provenance().delta_t_predictions_sha256(),
        eop_finals_sha: data.provenance().eop_finals_sha256(),
    }
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

fn apply_bundle_to_paths(
    data: &tempoch_time_data::TimeDataBundle,
    check_only: bool,
    paths: &OutputPaths,
) -> Result<i32, String> {
    let sources = bundled_sources();
    let provenance = bundle_provenance(data);

    let rendered_time = render_generated_module(
        data.utc_tai_segments(),
        data.modern_delta_t_points(),
        data.modern_delta_t_observed_end_mjd(),
        &sources,
        &provenance,
    );
    let rendered_eop = render_eop_module(data.eop_points(), &sources, &provenance);

    if check_only {
        let mut stale = false;
        for (path, rendered) in [
            (&paths.time_data, &rendered_time),
            (&paths.eop_data, &rendered_eop),
        ] {
            let current = std::fs::read_to_string(path).unwrap_or_default();
            if &current != rendered {
                eprintln!("{} is out of date", path.display());
                stale = true;
            } else {
                println!("{} is up to date", path.display());
            }
        }

        if provenance_is_current(&paths.provenance_sidecar, &provenance) {
            println!("{} is up to date", paths.provenance_sidecar.display());
        } else {
            eprintln!("{} is out of date", paths.provenance_sidecar.display());
            stale = true;
        }
        return Ok(if stale { 1 } else { 0 });
    }

    let time_changed =
        write_if_changed(&paths.time_data, &rendered_time).map_err(|e| e.to_string())?;
    let eop_changed =
        write_if_changed(&paths.eop_data, &rendered_eop).map_err(|e| e.to_string())?;
    let provenance_changed = write_provenance(
        &paths.provenance_sidecar,
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
        paths.time_data.display(),
        tag(time_changed),
        data.utc_tai_segments().len(),
        data.modern_delta_t_points().len(),
        data.modern_delta_t_observed_end_mjd()
    );
    println!(
        "{} {} (EOP points={}, observed through MJD {}, last MJD {})",
        paths.eop_data.display(),
        tag(eop_changed),
        data.eop_points().len(),
        data.eop_observed_end_mjd()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string()),
        data.eop_end_mjd()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "none".to_string()),
    );
    println!(
        "{} {}",
        paths.provenance_sidecar.display(),
        tag(provenance_changed),
    );

    Ok(0)
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

    let paths = default_output_paths();
    let result = apply_bundle_to_paths(&data, check_only, &paths);
    let _ = std::fs::remove_dir_all(&temp_dir);
    result
}

fn print_usage() {
    eprintln!(
        "Usage: tempoch-time-data-updater [--check]\n\
         \n\
         Regenerate tempoch-time-data/src/generated/{{time_data,eop_data}}.rs and the\n\
         provenance sidecar from upstream sources.\n\
         With --check, exit non-zero if any committed generated file is out of date."
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliMode {
    Run { check_only: bool },
    Help,
}

fn parse_cli_args<I, S>(args: I) -> Result<CliMode, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut check = false;
    for arg in args {
        match arg.as_ref() {
            "--check" => check = true,
            "-h" | "--help" => return Ok(CliMode::Help),
            other => return Err(other.to_owned()),
        }
    }
    Ok(CliMode::Run { check_only: check })
}

fn main() -> ExitCode {
    let mode = match parse_cli_args(std::env::args().skip(1)) {
        Ok(mode) => mode,
        Err(other) => {
            eprintln!("unknown argument: {other}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    if mode == CliMode::Help {
        print_usage();
        return ExitCode::SUCCESS;
    }

    let CliMode::Run { check_only } = mode else {
        unreachable!("help mode handled above")
    };

    match run(check_only) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code.clamp(0, 255) as u8),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    fn sample_bundle(fetched_utc: &str) -> tempoch_time_data::TimeDataBundle {
        let segments = vec![
            tempoch_time_data::UtcTaiSegment {
                start_mjd: 37_300,
                end_mjd: Some(37_512),
                base_seconds: 1.422_818,
                reference_mjd: 37_300.0,
                slope_seconds_per_day: 0.001_296,
            },
            tempoch_time_data::UtcTaiSegment {
                start_mjd: 37_512,
                end_mjd: None,
                base_seconds: 10.0,
                reference_mjd: 37_512.0,
                slope_seconds_per_day: 0.0,
            },
        ];

        let modern_delta_t = vec![(60_000.0, 69.8), (60_001.0, 69.9)];

        let eop_points = vec![
            tempoch_time_data::EopPoint {
                mjd: 60_000,
                pm_observed: true,
                ut1_observed: true,
                nutation_observed: true,
                pm_xp_arcsec: Some(0.1),
                pm_yp_arcsec: Some(-0.1),
                ut1_minus_utc_seconds: -0.123,
                lod_milliseconds: Some(1.2),
                dx_milliarcsec: Some(0.0),
                dy_milliarcsec: Some(0.0),
            },
            tempoch_time_data::EopPoint {
                mjd: 60_001,
                pm_observed: false,
                ut1_observed: false,
                nutation_observed: false,
                pm_xp_arcsec: None,
                pm_yp_arcsec: None,
                ut1_minus_utc_seconds: -0.2,
                lod_milliseconds: None,
                dx_milliarcsec: None,
                dy_milliarcsec: None,
            },
        ];

        let provenance = tempoch_time_data::TimeDataProvenance::new(
            fetched_utc,
            "utc-sha",
            "obs-sha",
            "pred-sha",
            "eop-sha",
        );

        tempoch_time_data::TimeDataBundle::new(
            segments,
            modern_delta_t,
            60_000.0,
            eop_points,
            provenance,
        )
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tempoch_updater_test_{}_{}_{}",
            label,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create test temp directory");
        dir
    }

    #[test]
    fn parse_cli_args_accepts_supported_flags() {
        assert_eq!(
            parse_cli_args(["--check"]),
            Ok(CliMode::Run { check_only: true })
        );
        assert_eq!(parse_cli_args(["--check", "--help"]), Ok(CliMode::Help));
        assert_eq!(parse_cli_args(["-h"]), Ok(CliMode::Help));
        assert_eq!(
            parse_cli_args(std::iter::empty::<&str>()),
            Ok(CliMode::Run { check_only: false })
        );
        assert_eq!(parse_cli_args(["--bogus"]), Err("--bogus".to_string()));
    }

    #[test]
    fn write_if_changed_detects_changes() {
        let dir = unique_temp_dir("write_if_changed");
        let path = dir.join("data.txt");

        assert!(write_if_changed(&path, "alpha\n").unwrap());
        assert!(!write_if_changed(&path, "alpha\n").unwrap());
        assert!(write_if_changed(&path, "beta\n").unwrap());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn provenance_helpers_track_hashes_and_timestamp() {
        let dir = unique_temp_dir("provenance");
        let path = dir.join("time_data.provenance.json");

        let first = sample_bundle("2026-01-01T00:00:00");
        let first_prov = bundle_provenance(&first);

        assert!(write_provenance(&path, "2026-01-01T00:00:00", &first_prov).unwrap());
        assert!(provenance_is_current(&path, &first_prov));

        let second = sample_bundle("2026-02-02T00:00:00");
        let second_prov = bundle_provenance(&second);
        assert!(!write_provenance(&path, "2026-02-02T00:00:00", &second_prov).unwrap());
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"fetched_utc\": \"2026-01-01T00:00:00\""));

        let changed = tempoch_time_data::TimeDataBundle::new(
            second.utc_tai_segments().to_vec(),
            second.modern_delta_t_points().to_vec(),
            second.modern_delta_t_observed_end_mjd(),
            second.eop_points().to_vec(),
            tempoch_time_data::TimeDataProvenance::new(
                "2026-03-03T00:00:00",
                "utc-sha-new",
                "obs-sha",
                "pred-sha",
                "eop-sha",
            ),
        );
        let changed_prov = bundle_provenance(&changed);
        assert!(write_provenance(&path, "2026-03-03T00:00:00", &changed_prov).unwrap());
        assert!(provenance_is_current(&path, &changed_prov));
        let updated = fs::read_to_string(&path).unwrap();
        assert!(updated.contains("\"fetched_utc\": \"2026-03-03T00:00:00\""));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn apply_bundle_to_paths_handles_check_and_write_modes() {
        let dir = unique_temp_dir("apply_bundle");
        let paths = OutputPaths {
            time_data: dir.join("time_data.rs"),
            eop_data: dir.join("eop_data.rs"),
            provenance_sidecar: dir.join("time_data.provenance.json"),
        };
        let bundle = sample_bundle("2026-01-01T00:00:00");

        assert_eq!(apply_bundle_to_paths(&bundle, true, &paths).unwrap(), 1);
        assert_eq!(apply_bundle_to_paths(&bundle, false, &paths).unwrap(), 0);
        assert_eq!(apply_bundle_to_paths(&bundle, true, &paths).unwrap(), 0);

        fs::write(&paths.time_data, "stale").unwrap();
        assert_eq!(apply_bundle_to_paths(&bundle, true, &paths).unwrap(), 1);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn default_output_paths_target_generated_files() {
        let paths = default_output_paths();
        assert!(paths
            .time_data
            .ends_with("tempoch-time-data/src/generated/time_data.rs"));
        assert!(paths
            .eop_data
            .ends_with("tempoch-time-data/src/generated/eop_data.rs"));
        assert!(paths
            .provenance_sidecar
            .ends_with("tempoch-time-data/src/generated/time_data.provenance.json"));
    }

    #[test]
    fn provenance_hash_match_requires_all_hashes() {
        let bundle = sample_bundle("2026-01-01T00:00:00");
        let provenance = bundle_provenance(&bundle);
        let json = serde_json::json!({
            "fetched_utc": "2026-01-01T00:00:00",
            "utc_tai_sha256": "utc-sha",
            "delta_t_observed_sha256": "obs-sha",
            "delta_t_predictions_sha256": "pred-sha",
            "eop_finals_sha256": "eop-sha",
        });
        assert!(provenance_hashes_match(&json, &provenance));

        let mismatch = serde_json::json!({
            "fetched_utc": "2026-01-01T00:00:00",
            "utc_tai_sha256": "utc-sha",
            "delta_t_observed_sha256": "obs-sha",
            "delta_t_predictions_sha256": "pred-sha",
            "eop_finals_sha256": "different",
        });
        assert!(!provenance_hashes_match(&mismatch, &provenance));
    }
}
