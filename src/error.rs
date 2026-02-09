use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error)]
pub enum Error {
    #[error("ffmpeg not found in PATH — install it from https://ffmpeg.org")]
    FfmpegNotFound,

    #[error("ffprobe not found in PATH — install it from https://ffmpeg.org")]
    FfprobeNotFound,

    #[error("input file does not exist: {0}")]
    InputNotFound(PathBuf),

    #[error("ffprobe failed: {0}")]
    ProbeFailed(String),

    #[error("ffmpeg encoding failed: {0}")]
    EncodeFailed(String),

    #[error("could not reach target size after {0} attempts — video may be too long or complex")]
    TargetUnreachable(u32),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl std::fmt::Debug for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self}")
    }
}
