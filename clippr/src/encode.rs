use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

pub struct EncodeParams {
    pub width: u32,
    pub fps: u32,
    pub colors: u32,
    pub start_secs: f64,
    pub duration_secs: f64,
}

pub fn encode(input: &Path, output: &Path, params: &EncodeParams) -> Result<u64> {
    let filter = format!(
        "fps={fps},scale={width}:-1:flags=lanczos,\
         split[s0][s1];\
         [s0]palettegen=max_colors={colors}:stats_mode=diff[p];\
         [s1][p]paletteuse=dither=floyd_steinberg",
        fps = params.fps,
        width = params.width,
        colors = params.colors,
    );

    let mut command = Command::new("ffmpeg");
    command.args(["-y", "-ss", &format!("{:.3}", params.start_secs)]);
    command.args(["-t", &format!("{:.3}", params.duration_secs)]);
    command.args(["-i"]);
    command.arg(input);
    command.args(["-vf", &filter]);
    command.arg(output);

    let result = command.output().map_err(|_| Error::FfmpegNotFound)?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(Error::EncodeFailed(stderr.into_owned()));
    }

    let metadata = std::fs::metadata(output)?;
    Ok(metadata.len())
}
