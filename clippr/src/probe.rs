use crate::error::{Error, Result};
use serde::Deserialize;
use std::path::Path;
use std::process::Command;

pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub duration_secs: f64,
    pub framerate: f64,
}

#[derive(Deserialize)]
struct FfprobeOutput {
    streams: Vec<StreamInfo>,
    format: FormatInfo,
}

#[derive(Deserialize)]
struct StreamInfo {
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
}

#[derive(Deserialize)]
struct FormatInfo {
    duration: Option<String>,
}

fn parse_frame_rate(raw: &str) -> Option<f64> {
    let parts: Vec<&str> = raw.split('/').collect();
    if parts.len() == 2 {
        let numerator: f64 = parts[0].parse().ok()?;
        let denominator: f64 = parts[1].parse().ok()?;
        if denominator > 0.0 {
            return Some(numerator / denominator);
        }
    }
    raw.parse().ok()
}

pub fn probe(path: &Path) -> Result<VideoInfo> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate",
            "-show_entries",
            "format=duration",
        ])
        .arg(path)
        .output()
        .map_err(|_| Error::FfprobeNotFound)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::ProbeFailed(stderr.into_owned()));
    }

    let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)?;

    let stream = parsed
        .streams
        .first()
        .ok_or_else(|| Error::ProbeFailed("no video stream found".into()))?;

    let width = stream
        .width
        .ok_or_else(|| Error::ProbeFailed("missing width".into()))?;

    let height = stream
        .height
        .ok_or_else(|| Error::ProbeFailed("missing height".into()))?;

    let framerate = stream
        .r_frame_rate
        .as_deref()
        .and_then(parse_frame_rate)
        .unwrap_or(30.0);

    let duration_secs = parsed
        .format
        .duration
        .as_deref()
        .and_then(|duration| duration.parse::<f64>().ok())
        .ok_or_else(|| Error::ProbeFailed("missing duration".into()))?;

    Ok(VideoInfo {
        width,
        height,
        duration_secs,
        framerate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_integer_fraction() {
        let result = parse_frame_rate("30/1").unwrap();
        assert!((result - 30.0).abs() < 0.001);
    }

    #[test]
    fn parse_ntsc_fraction() {
        let result = parse_frame_rate("30000/1001").unwrap();
        assert!((result - 29.97).abs() < 0.01);
    }

    #[test]
    fn parse_plain_float() {
        let result = parse_frame_rate("29.97").unwrap();
        assert!((result - 29.97).abs() < 0.001);
    }

    #[test]
    fn parse_plain_integer() {
        let result = parse_frame_rate("60").unwrap();
        assert!((result - 60.0).abs() < 0.001);
    }

    #[test]
    fn parse_zero_denominator_falls_back() {
        assert!(parse_frame_rate("30/0").is_none());
    }

    #[test]
    fn parse_garbage_returns_none() {
        assert!(parse_frame_rate("not_a_number").is_none());
    }

    #[test]
    fn parse_empty_returns_none() {
        assert!(parse_frame_rate("").is_none());
    }

    #[test]
    fn parse_fraction_with_garbage_numerator() {
        assert!(parse_frame_rate("abc/1").is_none());
    }

    #[test]
    fn parse_fraction_with_garbage_denominator() {
        assert!(parse_frame_rate("30/abc").is_none());
    }
}
