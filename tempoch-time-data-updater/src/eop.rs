// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Valles Puig, Ramon
//
// Parser for the IERS `finals2000A.all` fixed-width Earth Orientation
// Parameter file.  Each data line follows the layout published at
// <https://datacenter.iers.org/versionMetadata.php?filename=latestVersion/9_FINALS.ALL_IAU2000_V2013_019.txt>.
//
// Relevant columns (1-based, inclusive):
//
//      8-15  F8.2    Modified Julian Date (UTC)
//     17     A1      Bull. A PM flag: 'I' = observed, 'P' = prediction
//     19-27  F9.6    Bull. A PM-x  (seconds of arc)
//     38-46  F9.6    Bull. A PM-y  (seconds of arc)
//     58     A1      Bull. A UT1-UTC flag
//     59-68  F10.7   Bull. A UT1-UTC  (seconds of time)
//     80-86  F7.4    Bull. A LOD  (milliseconds of time, optional)
//     96     A1      Bull. A nutation flag
//     98-106 F9.3    Bull. A dX (milliarcseconds, IAU 2000A)
//    117-125 F9.3    Bull. A dY (milliarcseconds, IAU 2000A)
//
// Any line without at least a parseable MJD + UT1-UTC is skipped.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EopPoint {
    pub mjd: i32,
    pub pm_observed: bool,
    pub ut1_observed: bool,
    pub nutation_observed: bool,
    pub pm_xp_arcsec: Option<f64>,
    pub pm_yp_arcsec: Option<f64>,
    pub ut1_minus_utc_seconds: f64,
    /// `Some` only when the source row has a parseable LOD value. LOD is
    /// optional for predictions and may also be blank at the end of the
    /// observed series.
    pub lod_milliseconds: Option<f64>,
    pub dx_milliarcsec: Option<f64>,
    pub dy_milliarcsec: Option<f64>,
}

fn col(line: &str, start_1based: usize, end_1based_inclusive: usize) -> Option<&str> {
    let start = start_1based.checked_sub(1)?;
    let end = end_1based_inclusive;
    if line.len() < end {
        return None;
    }
    Some(&line[start..end])
}

fn parse_f64(slice: &str) -> Option<f64> {
    let trimmed = slice.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse::<f64>().ok()
}

fn parse_flag(slice: &str) -> Option<char> {
    slice.trim().chars().next()
}

/// Parse `finals2000A.all` text. Unparseable or partial lines are silently
/// skipped.  The result preserves the source row order, which is monotone in
/// MJD.
pub fn parse_eop_finals(text: &str) -> Result<Vec<EopPoint>, String> {
    let mut points: Vec<EopPoint> = Vec::new();

    for line in text.lines() {
        // Minimum row length up to the UT1-UTC value column.
        if line.len() < 68 {
            continue;
        }
        let Some(mjd_f) = col(line, 8, 15).and_then(parse_f64) else {
            continue;
        };
        let mjd = mjd_f.round() as i32;

        let Some(ut1_flag) = col(line, 58, 58).and_then(parse_flag) else {
            continue;
        };
        if !matches!(ut1_flag, 'I' | 'P') {
            continue;
        }
        let Some(ut1_minus_utc_seconds) = col(line, 59, 68).and_then(parse_f64) else {
            continue;
        };

        let pm_flag = col(line, 17, 17).and_then(parse_flag);
        let pm_xp_arcsec = col(line, 19, 27).and_then(parse_f64);
        let pm_yp_arcsec = col(line, 38, 46).and_then(parse_f64);

        let lod_milliseconds = col(line, 80, 86).and_then(parse_f64);

        let nutation_flag = col(line, 96, 96).and_then(parse_flag);
        let dx_milliarcsec = col(line, 98, 106).and_then(parse_f64);
        let dy_milliarcsec = col(line, 117, 125).and_then(parse_f64);

        points.push(EopPoint {
            mjd,
            pm_observed: matches!(pm_flag, Some('I')),
            ut1_observed: ut1_flag == 'I',
            nutation_observed: matches!(nutation_flag, Some('I')),
            pm_xp_arcsec,
            pm_yp_arcsec,
            ut1_minus_utc_seconds,
            lod_milliseconds,
            dx_milliarcsec,
            dy_milliarcsec,
        });
    }

    if points.len() < 2 {
        return Err("EOP finals parsing produced fewer than two usable rows".into());
    }

    // Sanity check: strictly increasing MJD.
    for window in points.windows(2) {
        if window[1].mjd <= window[0].mjd {
            return Err(format!(
                "EOP finals MJD column is not strictly increasing near {} → {}",
                window[0].mjd, window[1].mjd
            ));
        }
    }

    Ok(points)
}

/// Last MJD whose UT1-UTC came from an observation (flag 'I').
pub fn observed_end_mjd(points: &[EopPoint]) -> i32 {
    points
        .iter()
        .rev()
        .find(|p| p.ut1_observed)
        .map(|p| p.mjd)
        .unwrap_or(points[0].mjd)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic rows padded to the finals2000A.all column layout.
    // Columns are 1-based; we build a 200-char line explicitly.
    #[allow(clippy::too_many_arguments)]
    fn row(
        mjd: i32,
        pm_flag: char,
        xp: f64,
        yp: f64,
        ut1_flag: char,
        dut1: f64,
        lod: Option<f64>,
        nut_flag: char,
        dx: f64,
        dy: f64,
    ) -> String {
        let mut line = vec![b' '; 200];
        let write = |buf: &mut [u8], start: usize, end: usize, s: &str| {
            let len = end - start + 1;
            let b = format!("{s:>width$}", width = len);
            let b = b.as_bytes();
            buf[start - 1..start - 1 + len.min(b.len())].copy_from_slice(&b[..len.min(b.len())]);
        };
        write(&mut line, 8, 15, &format!("{:.2}", mjd as f64));
        line[16] = pm_flag as u8;
        write(&mut line, 19, 27, &format!("{xp:.6}"));
        write(&mut line, 38, 46, &format!("{yp:.6}"));
        line[57] = ut1_flag as u8;
        write(&mut line, 59, 68, &format!("{dut1:.7}"));
        if let Some(v) = lod {
            write(&mut line, 80, 86, &format!("{v:.4}"));
        }
        line[95] = nut_flag as u8;
        write(&mut line, 98, 106, &format!("{dx:.3}"));
        write(&mut line, 117, 125, &format!("{dy:.3}"));
        String::from_utf8(line).unwrap()
    }

    #[test]
    fn parses_observed_row_with_lod() {
        let text = row(
            60000,
            'I',
            0.123456,
            -0.234567,
            'I',
            -0.1234567,
            Some(1.2345),
            'I',
            0.100,
            -0.200,
        );
        let more = row(60001, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let input = format!("{text}\n{more}\n");
        let points = parse_eop_finals(&input).unwrap();
        assert_eq!(points.len(), 2);
        let p = points[0];
        assert_eq!(p.mjd, 60000);
        assert!(p.pm_observed);
        assert!(p.ut1_observed);
        assert!(p.nutation_observed);
        assert!((p.pm_xp_arcsec.unwrap() - 0.123456).abs() < 1e-9);
        assert!((p.pm_yp_arcsec.unwrap() + 0.234567).abs() < 1e-9);
        assert!((p.ut1_minus_utc_seconds + 0.1234567).abs() < 1e-12);
        assert!((p.lod_milliseconds.unwrap() - 1.2345).abs() < 1e-9);
        assert!((p.dx_milliarcsec.unwrap() - 0.100).abs() < 1e-9);
        assert!((p.dy_milliarcsec.unwrap() + 0.200).abs() < 1e-9);
    }

    #[test]
    fn parses_prediction_row_without_lod() {
        let obs = row(60000, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let pred = row(60001, 'P', 0.1, 0.1, 'P', -0.2, None, 'P', 0.0, 0.0);
        let input = format!("{obs}\n{pred}\n");
        let points = parse_eop_finals(&input).unwrap();
        assert_eq!(points.len(), 2);
        let p = points[1];
        assert!(!p.pm_observed);
        assert!(!p.ut1_observed);
        assert!(!p.nutation_observed);
        assert_eq!(p.lod_milliseconds, None);
        assert!((p.ut1_minus_utc_seconds + 0.2).abs() < 1e-12);
    }

    #[test]
    fn blank_optional_fields_stay_missing() {
        let obs = row(60000, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let mut blank = vec![b' '; 200];
        let write = |buf: &mut [u8], start: usize, end: usize, s: &str| {
            let len = end - start + 1;
            let b = format!("{s:>width$}", width = len);
            let b = b.as_bytes();
            buf[start - 1..start - 1 + len.min(b.len())].copy_from_slice(&b[..len.min(b.len())]);
        };
        write(&mut blank, 8, 15, "60001.00");
        blank[57] = b'P';
        write(&mut blank, 59, 68, "-0.2000000");
        let blank = String::from_utf8(blank).unwrap();

        let input = format!("{obs}\n{blank}\n");
        let points = parse_eop_finals(&input).unwrap();
        let p = points[1];
        assert_eq!(p.pm_xp_arcsec, None);
        assert_eq!(p.pm_yp_arcsec, None);
        assert_eq!(p.dx_milliarcsec, None);
        assert_eq!(p.dy_milliarcsec, None);
    }

    #[test]
    fn skips_short_or_blank_lines() {
        let good = row(60000, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let good2 = row(60001, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let input = format!("\n   \n# comment\nsomething too short\n{good}\n{good2}\n");
        let points = parse_eop_finals(&input).unwrap();
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn observed_end_is_last_observed_mjd() {
        let a = row(60000, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let b = row(60001, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let c = row(60002, 'P', 0.1, 0.1, 'P', -0.2, None, 'P', 0.0, 0.0);
        let input = format!("{a}\n{b}\n{c}\n");
        let points = parse_eop_finals(&input).unwrap();
        assert_eq!(observed_end_mjd(&points), 60001);
    }

    #[test]
    fn rejects_non_monotone_mjd() {
        let a = row(60000, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let b = row(60000, 'I', 0.1, 0.1, 'I', -0.1, Some(1.0), 'I', 0.1, 0.1);
        let input = format!("{a}\n{b}\n");
        assert!(parse_eop_finals(&input).is_err());
    }
}
