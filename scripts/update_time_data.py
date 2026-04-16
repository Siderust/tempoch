#!/usr/bin/env python3
"""Refresh generated time-data tables from official sources."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from datetime import datetime, timezone
from datetime import date
from hashlib import sha256
from pathlib import Path
import re
import sys
import urllib.request


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "tempoch-core" / "src" / "generated" / "time_data.rs"

UTC_TAI_HISTORY_URL = "https://hpiers.obspm.fr/eoppc/bul/bulc/UTC-TAI.history"
DELTA_T_OBSERVED_URL = "https://maia.usno.navy.mil/ser7/deltat.data"
DELTA_T_PREDICTIONS_URL = "https://maia.usno.navy.mil/ser7/deltat.preds"

PRE_1961_TAI_MINUS_UTC_APPROX = 10.0

MJD_EPOCH = date(1858, 11, 17)

MONTHS = {
    "jan": 1,
    "january": 1,
    "feb": 2,
    "february": 2,
    "mar": 3,
    "march": 3,
    "apr": 4,
    "april": 4,
    "may": 5,
    "jun": 6,
    "june": 6,
    "jul": 7,
    "july": 7,
    "aug": 8,
    "august": 8,
    "sep": 9,
    "sept": 9,
    "september": 9,
    "oct": 10,
    "october": 10,
    "nov": 11,
    "november": 11,
    "dec": 12,
    "december": 12,
}


@dataclass(frozen=True)
class UtcTaiSegment:
    start_mjd: int
    end_mjd: int | None
    base_seconds: float
    reference_mjd: float
    slope_seconds_per_day: float


def fetch_text(url: str) -> tuple[str, str]:
    """Fetch *url* and return ``(text, sha256_hex)``."""
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "tempoch-time-data-updater/1.0"},
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        raw = response.read()
    return raw.decode("utf-8"), sha256(raw).hexdigest()


def mjd_from_date(value: date) -> int:
    return (value - MJD_EPOCH).days


def normalize_whitespace(value: str) -> str:
    return " ".join(value.replace("\t", " ").split())


def parse_month(token: str) -> int:
    key = re.sub(r"[^a-z]", "", token.lower())
    if key not in MONTHS:
        raise ValueError(f"unknown month token: {token!r}")
    return MONTHS[key]


def parse_date_fragment(fragment: str, default_year: int | None) -> date:
    normalized = normalize_whitespace(fragment).rstrip(".")
    match = re.fullmatch(r"(?:(\d{4})\s+)?([A-Za-z.]+)\s+(\d+)", normalized)
    if not match:
        raise ValueError(f"unable to parse date fragment: {fragment!r}")

    year_text, month_text, day_text = match.groups()
    year = int(year_text) if year_text is not None else default_year
    if year is None:
        raise ValueError(f"missing year for fragment: {fragment!r}")
    month = parse_month(month_text)
    day = int(day_text)
    return date(year, month, day)


def compact_number(value: str) -> float:
    return float(value.replace(" ", ""))


def parse_utc_tai_segments(text: str) -> list[UtcTaiSegment]:
    segments: list[UtcTaiSegment] = []
    previous_end: date | None = None
    previous_reference_mjd: float | None = None
    previous_slope: float | None = None

    for raw_line in text.splitlines():
        line = raw_line.rstrip()
        if "-" not in line or "UTC-TAI.history" in line or "Limits of validity" in line:
            continue
        if not re.search(r"\d", line):
            continue

        left, right = line.split("-", 1)
        if not re.search(r"[A-Za-z]", left):
            continue

        default_start_year = previous_end.year if previous_end is not None else None
        start_date = parse_date_fragment(left, default_start_year)

        right_normalized = normalize_whitespace(right)
        end_match = re.match(r"^(?:(\d{4})\s+)?([A-Za-z.]+)\s+(\d+)\s+(.*)$", right_normalized)
        if end_match:
            end_year_text, end_month_text, end_day_text, formula = end_match.groups()
            end_year = int(end_year_text) if end_year_text is not None else start_date.year
            end_date: date | None = date(end_year, parse_month(end_month_text), int(end_day_text))
        else:
            end_date = None
            formula = right_normalized

        base_match = re.search(r"([0-9][0-9.\s]*)s", formula)
        if base_match is None:
            raise ValueError(f"unable to parse TAI-UTC base from {formula!r}")
        base_seconds = compact_number(base_match.group(1))

        slope_match = re.search(r"\(MJD\s*-\s*([0-9\s]+)\)\s*x\s*([0-9.\s]+)s", formula)
        if slope_match is not None:
            reference_mjd = compact_number(slope_match.group(1))
            slope_seconds_per_day = compact_number(slope_match.group(2))
        elif '""' in formula:
            if previous_reference_mjd is None or previous_slope is None:
                raise ValueError(f"repeated UTC formula without previous state: {formula!r}")
            reference_mjd = previous_reference_mjd
            slope_seconds_per_day = previous_slope
        else:
            reference_mjd = float(mjd_from_date(start_date))
            slope_seconds_per_day = 0.0

        segments.append(
            UtcTaiSegment(
                start_mjd=mjd_from_date(start_date),
                end_mjd=mjd_from_date(end_date) if end_date is not None else None,
                base_seconds=base_seconds,
                reference_mjd=reference_mjd,
                slope_seconds_per_day=slope_seconds_per_day,
            )
        )

        previous_end = end_date
        previous_reference_mjd = reference_mjd
        previous_slope = slope_seconds_per_day

    if not segments:
        raise ValueError("UTC-TAI history parsing produced no segments")

    return segments


def parse_delta_t_observed(text: str) -> list[tuple[float, float]]:
    points: list[tuple[float, float]] = []
    for raw_line in text.splitlines():
        parts = raw_line.split()
        if len(parts) != 4 or not parts[0].isdigit():
            continue
        year, month, day = (int(parts[0]), int(parts[1]), int(parts[2]))
        delta_t_seconds = float(parts[3])
        points.append((float(mjd_from_date(date(year, month, day))), delta_t_seconds))

    if not points:
        raise ValueError("observed Delta T parsing produced no points")

    return points


def parse_delta_t_predictions(text: str) -> list[tuple[float, float]]:
    points: list[tuple[float, float]] = []
    for raw_line in text.splitlines():
        parts = raw_line.split()
        if not parts or parts[0] == "MJD":
            continue
        if len(parts) < 3:
            continue
        points.append((float(parts[0]), float(parts[2])))

    if not points:
        raise ValueError("predicted Delta T parsing produced no points")

    return points


def build_modern_delta_t_points(
    observed_points: list[tuple[float, float]],
    predicted_points: list[tuple[float, float]],
) -> tuple[list[tuple[float, float]], float]:
    last_observed_mjd, last_observed_dt = observed_points[-1]
    future_predictions = [point for point in predicted_points if point[0] > last_observed_mjd]

    if future_predictions:
        # Enforce C0 continuity at the stitch: interpolate the prediction model
        # back to last_observed_mjd and compute a constant offset so the first
        # prediction point meets the last observed value exactly.
        m0, d0 = future_predictions[0]
        if len(future_predictions) >= 2:
            m1, d1 = future_predictions[1]
        else:
            m1, d1 = m0, d0
        frac = (last_observed_mjd - m0) / (m1 - m0) if m1 != m0 else 0.0
        pred_at_stitch = d0 + frac * (d1 - d0)
        continuity_offset = last_observed_dt - pred_at_stitch
        future_predictions = [(m, d + continuity_offset) for m, d in future_predictions]

    combined = observed_points + future_predictions
    if len(combined) < 2:
        raise ValueError("modern Delta T series must contain at least two points")
    return combined, last_observed_mjd


def render_segments(segments: list[UtcTaiSegment]) -> str:
    rendered = []
    for segment in segments:
        end_value = "None" if segment.end_mjd is None else f"Some({segment.end_mjd})"
        rendered.append(
            "    UtcTaiSegment {\n"
            f"        start_mjd: {segment.start_mjd},\n"
            f"        end_mjd: {end_value},\n"
            f"        base_seconds: {segment.base_seconds:.7f},\n"
            f"        reference_mjd: {segment.reference_mjd:.1f},\n"
            f"        slope_seconds_per_day: {segment.slope_seconds_per_day:.7f},\n"
            "    },"
        )
    return "\n".join(rendered)


def render_points(points: list[tuple[float, float]]) -> str:
    return "\n".join(f"    ({mjd:.3f}, {delta_t:.4f})," for mjd, delta_t in points)


def render_generated_module(
    utc_tai_segments: list[UtcTaiSegment],
    modern_delta_t_points: list[tuple[float, float]],
    observed_end_mjd: float,
    provenance: dict[str, tuple[str, str]],
) -> str:
    utc_history_start = utc_tai_segments[0].start_mjd
    modern_delta_t_start = modern_delta_t_points[0][0]
    modern_delta_t_end = modern_delta_t_points[-1][0]

    _, utc_tai_sha = provenance["utc_tai"]
    _, delta_t_obs_sha = provenance["delta_t_observed"]
    _, delta_t_pred_sha = provenance["delta_t_predictions"]

    return f"""// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon
//
// @generated by scripts/update_time_data.py
// Do not edit this file manually.
//
// Provenance (run `python3 scripts/update_time_data.py --check` to verify):
//   UTC-TAI history  SHA-256: {utc_tai_sha}
//   ΔT observed      SHA-256: {delta_t_obs_sha}
//   ΔT predictions   SHA-256: {delta_t_pred_sha}
//
// The modern ΔT series is a *derived* product: observed USNO monthly points
// are concatenated with C0-adjusted prediction points.  The boundary MJD
// between observed and predicted data is MODERN_DELTA_T_OBSERVED_END_MJD.

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct UtcTaiSegment {{
    pub start_mjd: i32,
    pub end_mjd: Option<i32>,
    pub base_seconds: f64,
    pub reference_mjd: f64,
    pub slope_seconds_per_day: f64,
}}

pub(crate) const PRE_1961_TAI_MINUS_UTC_APPROX: f64 = {PRE_1961_TAI_MINUS_UTC_APPROX:.1f};
pub(crate) const UTC_TAI_HISTORY_URL: &str = "{UTC_TAI_HISTORY_URL}";
pub(crate) const DELTA_T_OBSERVED_URL: &str = "{DELTA_T_OBSERVED_URL}";
pub(crate) const DELTA_T_PREDICTIONS_URL: &str = "{DELTA_T_PREDICTIONS_URL}";
pub(crate) const UTC_TAI_HISTORY_START_MJD: i32 = {utc_history_start};
pub(crate) const MODERN_DELTA_T_START_MJD: f64 = {modern_delta_t_start:.3f};
/// Last MJD with an *observed* (non-predicted) ΔT value in [`MODERN_DELTA_T_POINTS`].
/// Points after this MJD are C0-adjusted USNO predictions, not confirmed observations.
pub(crate) const MODERN_DELTA_T_OBSERVED_END_MJD: f64 = {observed_end_mjd:.3f};
pub(crate) const MODERN_DELTA_T_END_MJD: f64 = {modern_delta_t_end:.3f};

#[rustfmt::skip]
pub(crate) const UTC_TAI_SEGMENTS: [UtcTaiSegment; {len(utc_tai_segments)}] = [
{render_segments(utc_tai_segments)}
];

#[rustfmt::skip]
pub(crate) const MODERN_DELTA_T_POINTS: [(f64, f64); {len(modern_delta_t_points)}] = [
{render_points(modern_delta_t_points)}
];
"""


def write_output(contents: str) -> bool:
    existing = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else None
    if existing == contents:
        return False
    OUTPUT.write_text(contents, encoding="utf-8")
    return True


PROVENANCE_SIDECAR = OUTPUT.with_suffix(".provenance.json")


def write_provenance_sidecar(provenance: dict[str, tuple[str, str]]) -> None:
    """Write fetch timestamp + SHA-256s to a sidecar JSON (not embedded in the .rs)."""
    import json

    data = {
        "fetched_utc": provenance["utc_tai"][0],
        "utc_tai_sha256": provenance["utc_tai"][1],
        "delta_t_observed_sha256": provenance["delta_t_observed"][1],
        "delta_t_predictions_sha256": provenance["delta_t_predictions"][1],
    }
    PROVENANCE_SIDECAR.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the generated file is stale",
    )
    args = parser.parse_args(argv)

    fetch_ts = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S")
    utc_tai_history, utc_tai_sha = fetch_text(UTC_TAI_HISTORY_URL)
    delta_t_observed, delta_t_obs_sha = fetch_text(DELTA_T_OBSERVED_URL)
    delta_t_predictions, delta_t_pred_sha = fetch_text(DELTA_T_PREDICTIONS_URL)

    provenance = {
        "utc_tai": (fetch_ts, utc_tai_sha),
        "delta_t_observed": (fetch_ts, delta_t_obs_sha),
        "delta_t_predictions": (fetch_ts, delta_t_pred_sha),
    }

    utc_tai_segments = parse_utc_tai_segments(utc_tai_history)
    modern_delta_t_points, observed_end_mjd = build_modern_delta_t_points(
        parse_delta_t_observed(delta_t_observed),
        parse_delta_t_predictions(delta_t_predictions),
    )
    rendered = render_generated_module(
        utc_tai_segments, modern_delta_t_points, observed_end_mjd, provenance
    )

    if args.check:
        current = OUTPUT.read_text(encoding="utf-8") if OUTPUT.exists() else ""
        if current != rendered:
            print(f"{OUTPUT} is out of date", file=sys.stderr)
            return 1
        print(f"{OUTPUT} is up to date")
        return 0

    changed = write_output(rendered)
    write_provenance_sidecar(provenance)
    status = "updated" if changed else "already current"
    print(
        f"{OUTPUT} {status} "
        f"(UTC-TAI segments={len(utc_tai_segments)}, modern Delta T points={len(modern_delta_t_points)}, "
        f"observed through MJD {observed_end_mjd:.0f})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
