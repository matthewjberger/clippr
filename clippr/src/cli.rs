use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "clippr", about = "Convert MP4 to chunked GitHub-friendly GIFs")]
pub struct Cli {
    pub input: Option<PathBuf>,

    #[arg(
        short,
        long,
        help = "Output path (without extension; chunks get numbered suffixes)"
    )]
    pub output: Option<PathBuf>,

    #[arg(long, default_value = "10", help = "Max file size per chunk in MB")]
    pub max_size_mb: f64,

    #[arg(long, default_value = "480", help = "Starting width in pixels")]
    pub width: u32,

    #[arg(long, default_value = "15", help = "Starting frames per second")]
    pub fps: u32,

    #[arg(long, default_value = "256", help = "Starting palette color count")]
    pub colors: u32,

    #[arg(
        long,
        default_value = "3.0",
        help = "Duration of each chunk in seconds"
    )]
    pub chunk_secs: f64,

    #[arg(long, help = "Launch the graphical interface")]
    pub gui: bool,
}
