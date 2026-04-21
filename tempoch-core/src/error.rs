// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Vallés Puig, Ramon

/// Conversion error surface.
///
/// Variants are payload-free in v1 to keep the matrix small; they may carry
/// context in later phases if a concrete call-site demands it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionError {
    /// The active UTC history or policy cannot represent the requested date.
    UtcHistoryUnsupported,
    /// A leap-second label does not correspond to a leap second in the
    /// compiled UTC history.
    InvalidLeapSecond,
    /// The converted value leaves the representable range of the target.
    OutOfRange,
    /// A UT1 conversion was requested outside the horizon of the configured
    /// ΔT model or observed-data source.
    Ut1HorizonExceeded,
    /// Input or arithmetic produced `NaN` or `±∞`.
    NonFinite,
}

impl core::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UtcHistoryUnsupported => {
                f.write_str("UTC history is unavailable for the requested date")
            }
            Self::InvalidLeapSecond => {
                f.write_str("leap-second label is not present in the compiled UTC history")
            }
            Self::OutOfRange => f.write_str("converted value is out of representable range"),
            Self::Ut1HorizonExceeded => {
                f.write_str("UT1 conversion exceeds the ΔT model or data horizon")
            }
            Self::NonFinite => f.write_str("time value must be finite (not NaN or infinity)"),
        }
    }
}

impl std::error::Error for ConversionError {}

/// Error surface for runtime time-data operations.
///
/// Returned by [`crate::update_runtime_time_data`] and
/// [`crate::refresh_runtime_time_data`] when the runtime data bundle cannot be
/// loaded or refreshed.
#[derive(Debug)]
pub enum TimeDataError {
    /// An I/O error occurred while reading or writing the data bundle.
    Io(std::io::Error),
    /// A network download failed.
    Download(String),
    /// The data could not be parsed.
    Parse(String),
    /// The data bundle failed an integrity check.
    Integrity(String),
}

impl core::fmt::Display for TimeDataError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Download(msg) => write!(f, "download error: {msg}"),
            Self::Parse(msg) => write!(f, "parse error: {msg}"),
            Self::Integrity(msg) => write!(f, "integrity error: {msg}"),
        }
    }
}

impl std::error::Error for TimeDataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TimeDataError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<tempoch_time_data::TimeDataError> for TimeDataError {
    fn from(err: tempoch_time_data::TimeDataError) -> Self {
        match err {
            tempoch_time_data::TimeDataError::Io(e) => Self::Io(e),
            tempoch_time_data::TimeDataError::Download(msg) => Self::Download(msg),
            tempoch_time_data::TimeDataError::Parse(msg) => Self::Parse(msg),
            tempoch_time_data::TimeDataError::Integrity(msg) => Self::Integrity(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn io_error(msg: &str) -> std::io::Error {
        std::io::Error::other(msg.to_string())
    }

    #[test]
    fn display_all_variants() {
        let cases: &[(ConversionError, &str)] = &[
            (ConversionError::UtcHistoryUnsupported, "history"),
            (ConversionError::InvalidLeapSecond, "leap-second"),
            (ConversionError::OutOfRange, "range"),
            (ConversionError::Ut1HorizonExceeded, "horizon"),
            (ConversionError::NonFinite, "finite"),
        ];
        for (variant, fragment) in cases {
            let s = variant.to_string();
            assert!(s.contains(fragment), "{variant:?}: got {s:?}");
        }
    }

    #[test]
    fn time_data_error_display_and_source() {
        let io = TimeDataError::Io(io_error("disk full"));
        assert!(io.to_string().contains("I/O error: disk full"));
        assert!(io.source().is_some());

        let download = TimeDataError::Download("network timeout".to_string());
        assert!(download
            .to_string()
            .contains("download error: network timeout"));
        assert!(download.source().is_none());

        let parse = TimeDataError::Parse("bad row".to_string());
        assert!(parse.to_string().contains("parse error: bad row"));
        assert!(parse.source().is_none());

        let integrity = TimeDataError::Integrity("checksum mismatch".to_string());
        assert!(integrity
            .to_string()
            .contains("integrity error: checksum mismatch"));
        assert!(integrity.source().is_none());
    }

    #[test]
    fn time_data_error_from_mappings_cover_all_variants() {
        let io_mapped: TimeDataError = io_error("io map").into();
        assert!(matches!(io_mapped, TimeDataError::Io(_)));

        let mapped_download: TimeDataError =
            tempoch_time_data::TimeDataError::Download("d".to_string()).into();
        assert!(matches!(mapped_download, TimeDataError::Download(msg) if msg == "d"));

        let mapped_parse: TimeDataError =
            tempoch_time_data::TimeDataError::Parse("p".to_string()).into();
        assert!(matches!(mapped_parse, TimeDataError::Parse(msg) if msg == "p"));

        let mapped_integrity: TimeDataError =
            tempoch_time_data::TimeDataError::Integrity("i".to_string()).into();
        assert!(matches!(mapped_integrity, TimeDataError::Integrity(msg) if msg == "i"));

        let mapped_io: TimeDataError = tempoch_time_data::TimeDataError::Io(io_error("x")).into();
        assert!(matches!(mapped_io, TimeDataError::Io(_)));
    }
}
