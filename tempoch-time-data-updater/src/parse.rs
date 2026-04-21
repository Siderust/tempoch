// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon

use chrono::NaiveDate;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtcTaiSegment {
    pub start_mjd: i32,
    pub end_mjd: Option<i32>,
    pub base_seconds: f64,
    pub reference_mjd: f64,
    pub slope_seconds_per_day: f64,
}

fn mjd_epoch() -> NaiveDate {
    NaiveDate::from_ymd_opt(1858, 11, 17).unwrap()
}

pub fn mjd_from_date(d: NaiveDate) -> i32 {
    (d - mjd_epoch()).num_days() as i32
}

fn normalize_ws(s: &str) -> String {
    s.replace('\t', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_month(token: &str) -> Result<u32, String> {
    let key: String = token
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let m = match key.as_str() {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "sept" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return Err(format!("unknown month token: {token:?}")),
    };
    Ok(m)
}

/// Parse "YYYY Mon D" or "Mon D" (possibly with trailing period) into a date.
fn parse_date_fragment(fragment: &str, default_year: Option<i32>) -> Result<NaiveDate, String> {
    let normalized = normalize_ws(fragment);
    let normalized = normalized.trim_end_matches('.').trim();

    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    let (year, month_tok, day_tok) = match tokens.as_slice() {
        [y, m, d] if y.chars().all(|c| c.is_ascii_digit()) && y.len() == 4 => {
            (y.parse::<i32>().map_err(|e| e.to_string())?, *m, *d)
        }
        [m, d] => (
            default_year.ok_or_else(|| format!("missing year for fragment: {fragment:?}"))?,
            *m,
            *d,
        ),
        _ => return Err(format!("unable to parse date fragment: {fragment:?}")),
    };

    let month = parse_month(month_tok)?;
    let day: u32 = day_tok
        .parse()
        .map_err(|_| format!("bad day in fragment: {fragment:?}"))?;
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| format!("invalid calendar date in fragment: {fragment:?}"))
}

fn compact_number(s: &str) -> Result<f64, String> {
    s.replace(' ', "")
        .parse::<f64>()
        .map_err(|e| format!("bad number {s:?}: {e}"))
}

/// Extract `<base>s` where `<base>` is digits/spaces/dots, returning (base, rest_after_s).
fn extract_base_seconds(formula: &str) -> Result<f64, String> {
    // Find first 's' whose preceding run contains a digit.
    let bytes = formula.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b's' {
            // Walk back over digits/dots/spaces.
            let mut start = i;
            while start > 0 {
                let c = bytes[start - 1];
                if c.is_ascii_digit() || c == b'.' || c == b' ' {
                    start -= 1;
                } else {
                    break;
                }
            }
            let candidate = &formula[start..i];
            if candidate.chars().any(|c| c.is_ascii_digit()) {
                return compact_number(candidate);
            }
        }
        i += 1;
    }
    Err(format!("unable to parse TAI-UTC base from {formula:?}"))
}

/// Extract `(MJD - <ref>) x <slope>s` if present.
fn extract_slope(formula: &str) -> Result<Option<(f64, f64)>, String> {
    let Some(mjd_idx) = formula.find("MJD") else {
        return Ok(None);
    };
    let rest = &formula[mjd_idx + 3..];
    // Expect optional ws, '-', optional ws, <digits/spaces>, ')', optional ws, 'x', optional ws, <number>, 's'
    let rest = rest.trim_start();
    if !rest.starts_with('-') {
        return Ok(None);
    }
    let after_dash = rest[1..].trim_start();
    // Take digits+spaces as reference mjd.
    let ref_end = after_dash
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == ' '))
        .map(|(i, _)| i)
        .unwrap_or(after_dash.len());
    let ref_str = after_dash[..ref_end].trim();
    if ref_str.is_empty() {
        return Ok(None);
    }
    let reference_mjd = compact_number(ref_str)?;
    let after_ref = after_dash[ref_end..].trim_start();
    let after_paren = after_ref
        .strip_prefix(')')
        .unwrap_or(after_ref)
        .trim_start();
    let after_x = match after_paren.strip_prefix('x') {
        Some(r) => r.trim_start(),
        None => return Ok(None),
    };
    // Take digits/dots/spaces for slope.
    let slope_end = after_x
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || *c == '.' || *c == ' '))
        .map(|(i, _)| i)
        .unwrap_or(after_x.len());
    let slope_str = after_x[..slope_end].trim();
    if slope_str.is_empty() {
        return Ok(None);
    }
    let rest_after_slope = after_x[slope_end..].trim_start();
    if !rest_after_slope.starts_with('s') {
        return Ok(None);
    }
    let slope = compact_number(slope_str)?;
    Ok(Some((reference_mjd, slope)))
}

pub fn parse_utc_tai_segments(text: &str) -> Result<Vec<UtcTaiSegment>, String> {
    let mut segments: Vec<UtcTaiSegment> = Vec::new();
    let mut previous_end: Option<NaiveDate> = None;
    let mut previous_reference_mjd: Option<f64> = None;
    let mut previous_slope: Option<f64> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        if !line.contains('-')
            || line.contains("UTC-TAI.history")
            || line.contains("Limits of validity")
        {
            continue;
        }
        if !line.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }

        let dash_idx = line.find('-').unwrap();
        let (left, right) = line.split_at(dash_idx);
        let right = &right[1..]; // drop the '-'

        if !left.chars().any(|c| c.is_ascii_alphabetic()) {
            continue;
        }

        let default_start_year = previous_end.map(date_year);
        let start_date = parse_date_fragment(left, default_start_year)?;

        let right_normalized = normalize_ws(right);
        // Try to match: optional 4-digit year, month token, day, then formula.
        let (end_date, formula) = match parse_end_and_formula(&right_normalized, start_date) {
            Some((ed, f)) => (Some(ed), f),
            None => (None, right_normalized.clone()),
        };

        let base_seconds = extract_base_seconds(&formula)?;

        let (reference_mjd, slope_seconds_per_day) = if let Some((r, s)) = extract_slope(&formula)?
        {
            (r, s)
        } else if formula.contains("\"\"") {
            match (previous_reference_mjd, previous_slope) {
                (Some(r), Some(s)) => (r, s),
                _ => {
                    return Err(format!(
                        "repeated UTC formula without previous state: {formula:?}"
                    ))
                }
            }
        } else {
            (mjd_from_date(start_date) as f64, 0.0)
        };

        segments.push(UtcTaiSegment {
            start_mjd: mjd_from_date(start_date),
            end_mjd: end_date.map(mjd_from_date),
            base_seconds,
            reference_mjd,
            slope_seconds_per_day,
        });

        previous_end = end_date;
        previous_reference_mjd = Some(reference_mjd);
        previous_slope = Some(slope_seconds_per_day);
    }

    if segments.is_empty() {
        return Err("UTC-TAI history parsing produced no segments".into());
    }
    Ok(segments)
}

fn date_year(d: NaiveDate) -> i32 {
    use chrono::Datelike;
    d.year()
}

/// If `right_normalized` starts with `[YYYY ]Mon D <formula>`, return `(end_date, formula)`.
fn parse_end_and_formula(
    right_normalized: &str,
    start_date: NaiveDate,
) -> Option<(NaiveDate, String)> {
    let tokens: Vec<&str> = right_normalized.splitn(4, ' ').collect();
    if tokens.len() < 3 {
        return None;
    }
    // Case A: "YYYY Mon D rest"
    if tokens.len() == 4
        && tokens[0].len() == 4
        && tokens[0].chars().all(|c| c.is_ascii_digit())
        && parse_month(tokens[1]).is_ok()
        && !tokens[2].is_empty()
        && tokens[2].chars().all(|c| c.is_ascii_digit())
    {
        let year = tokens[0].parse::<i32>().ok()?;
        let month = parse_month(tokens[1]).ok()?;
        let day = tokens[2].parse::<u32>().ok()?;
        let ed = NaiveDate::from_ymd_opt(year, month, day)?;
        return Some((ed, tokens[3].to_string()));
    }
    // Case B: "Mon D rest" (use start_date.year)
    if parse_month(tokens[0]).is_ok()
        && tokens[1].chars().all(|c| c.is_ascii_digit())
        && !tokens[1].is_empty()
    {
        let month = parse_month(tokens[0]).ok()?;
        let day = tokens[1].parse::<u32>().ok()?;
        let ed = NaiveDate::from_ymd_opt(date_year(start_date), month, day)?;
        let rest = if tokens.len() >= 3 {
            right_normalized
                .splitn(3, ' ')
                .nth(2)
                .unwrap_or("")
                .to_string()
        } else {
            String::new()
        };
        return Some((ed, rest));
    }
    None
}

pub fn parse_delta_t_observed(text: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut points: Vec<(f64, f64)> = Vec::new();
    for raw_line in text.lines() {
        let parts: Vec<&str> = raw_line.split_whitespace().collect();
        if parts.len() != 4 {
            continue;
        }
        if !parts[0].chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let year: i32 = parts[0]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let month: u32 = parts[1]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let day: u32 = parts[2]
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        let dt: f64 = parts[3]
            .parse()
            .map_err(|e: std::num::ParseFloatError| e.to_string())?;
        let date = NaiveDate::from_ymd_opt(year, month, day)
            .ok_or_else(|| format!("invalid date in observed ΔT: {raw_line:?}"))?;
        points.push((mjd_from_date(date) as f64, dt));
    }
    if points.is_empty() {
        return Err("observed Delta T parsing produced no points".into());
    }
    Ok(points)
}

pub fn parse_delta_t_predictions(text: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut points: Vec<(f64, f64)> = Vec::new();
    for raw_line in text.lines() {
        let parts: Vec<&str> = raw_line.split_whitespace().collect();
        if parts.is_empty() || parts[0] == "MJD" {
            continue;
        }
        if parts.len() < 3 {
            continue;
        }
        let Ok(mjd) = parts[0].parse::<f64>() else {
            continue;
        };
        let Ok(dt) = parts[2].parse::<f64>() else {
            continue;
        };
        points.push((mjd, dt));
    }
    if points.is_empty() {
        return Err("predicted Delta T parsing produced no points".into());
    }
    Ok(points)
}

/// Stitch observed and predicted ΔT with C0-continuity offset. Returns (combined, last_observed_mjd).
pub fn build_modern_delta_t_points(
    observed_points: &[(f64, f64)],
    predicted_points: &[(f64, f64)],
) -> Result<(Vec<(f64, f64)>, f64), String> {
    let (last_obs_mjd, last_obs_dt) = *observed_points.last().ok_or("observed ΔT is empty")?;

    let mut future: Vec<(f64, f64)> = predicted_points
        .iter()
        .copied()
        .filter(|(m, _)| *m > last_obs_mjd)
        .collect();

    if !future.is_empty() {
        let (m0, d0) = future[0];
        let (m1, d1) = if future.len() >= 2 {
            future[1]
        } else {
            (m0, d0)
        };
        let frac = if m1 != m0 {
            (last_obs_mjd - m0) / (m1 - m0)
        } else {
            0.0
        };
        let pred_at_stitch = d0 + frac * (d1 - d0);
        let continuity_offset = last_obs_dt - pred_at_stitch;
        for p in future.iter_mut() {
            p.1 += continuity_offset;
        }
    }

    let mut combined = Vec::with_capacity(observed_points.len() + future.len());
    combined.extend_from_slice(observed_points);
    combined.extend_from_slice(&future);
    if combined.len() < 2 {
        return Err("modern Delta T series must contain at least two points".into());
    }
    Ok((combined, last_obs_mjd))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mjd_epoch_is_zero() {
        assert_eq!(
            mjd_from_date(NaiveDate::from_ymd_opt(1858, 11, 17).unwrap()),
            0
        );
    }

    #[test]
    fn parses_single_utc_tai_line_with_slope() {
        let text = "\
 1961  Jan.  1 - 1961  Aug.  1     1.422 818 0s + (MJD - 37 300) x 0.001 296s
";
        let segs = parse_utc_tai_segments(text).unwrap();
        assert_eq!(segs.len(), 1);
        let s = segs[0];
        assert_eq!(s.start_mjd, 37300);
        assert_eq!(s.end_mjd, Some(37512));
        assert!((s.base_seconds - 1.4228180).abs() < 1e-9);
        assert!((s.reference_mjd - 37300.0).abs() < 1e-9);
        assert!((s.slope_seconds_per_day - 0.001296).abs() < 1e-9);
    }

    #[test]
    fn parses_repeated_formula() {
        let text = "\
 1961  Jan.  1 - 1961  Aug.  1     1.422 818 0s + (MJD - 37 300) x 0.001 296s
 1961  Aug.  1 - 1962  Jan.  1     1.372 818 0s +       \"\"
";
        let segs = parse_utc_tai_segments(text).unwrap();
        assert_eq!(segs.len(), 2);
        let s = segs[1];
        assert!((s.base_seconds - 1.3728180).abs() < 1e-9);
        assert!((s.reference_mjd - 37300.0).abs() < 1e-9);
        assert!((s.slope_seconds_per_day - 0.001296).abs() < 1e-9);
    }

    #[test]
    fn parses_flat_leap_line_with_no_slope() {
        let text = "\
 1972  Jan.  1 - 1972  Jul.  1    10s
";
        let segs = parse_utc_tai_segments(text).unwrap();
        let s = segs[0];
        assert!((s.base_seconds - 10.0).abs() < 1e-12);
        assert_eq!(s.slope_seconds_per_day, 0.0);
    }

    #[test]
    fn observed_dt_parses_and_skips_junk() {
        let text = "\
# comment line
1973  2  1   43.4724
not a date
1973  3  1   43.5648
";
        let pts = parse_delta_t_observed(text).unwrap();
        assert_eq!(pts.len(), 2);
        assert!((pts[0].1 - 43.4724).abs() < 1e-9);
    }

    #[test]
    fn predicted_dt_parses_and_skips_header() {
        let text = "\
   MJD        YEAR    TT-UT Pred  UT1-UTC Pred  ERROR
61000.00     2025.50     69.80      -0.20         0.10
61100.00     2025.77     70.10      -0.30         0.12
";
        let pts = parse_delta_t_predictions(text).unwrap();
        assert_eq!(pts.len(), 2);
        assert!((pts[0].0 - 61000.0).abs() < 1e-9);
        assert!((pts[0].1 - 69.80).abs() < 1e-9);
    }

    #[test]
    fn build_modern_applies_c0_continuity() {
        let observed = vec![(60000.0, 69.5), (60100.0, 69.8)];
        let predicted = vec![(60050.0, 69.6), (60200.0, 70.0), (60300.0, 70.3)];
        let (combined, last_obs) = build_modern_delta_t_points(&observed, &predicted).unwrap();
        assert_eq!(last_obs, 60100.0);
        // First future point is 60200.0; should be shifted so linear interpolation of future
        // evaluated at 60100.0 equals 69.8.
        let (m0, d0) = (60200.0, 70.0);
        let (m1, d1) = (60300.0, 70.3);
        let frac = (60100.0 - m0) / (m1 - m0);
        let pred_at_stitch = d0 + frac * (d1 - d0);
        let offset = 69.8 - pred_at_stitch;
        assert!((combined[2].1 - (70.0 + offset)).abs() < 1e-12);
        assert!((combined[3].1 - (70.3 + offset)).abs() < 1e-12);
        assert_eq!(combined.len(), 4);
    }
}
