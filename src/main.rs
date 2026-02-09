mod cli;
mod encode;
mod error;
mod probe;
mod strategy;

use clap::Parser;
use cli::Cli;
use error::{Error, Result};
use std::path::{Path, PathBuf};
use strategy::InitialParams;

fn output_stem_from_args(input: &Path, output: Option<&Path>) -> Result<PathBuf> {
    match output {
        Some(path) => Ok(path.to_path_buf().with_extension("")),
        None => {
            let stem = input
                .file_stem()
                .ok_or_else(|| Error::InvalidInput("input has no file stem".into()))?;
            Ok(input.with_file_name(stem))
        }
    }
}

fn chunk_output_path(stem: &Path, chunk_index: u32, chunk_count: u32) -> PathBuf {
    if chunk_count == 1 {
        stem.with_extension("gif")
    } else {
        let name = format!(
            "{}_{:03}.gif",
            stem.file_name().unwrap_or_default().to_string_lossy(),
            chunk_index + 1,
        );
        stem.with_file_name(name)
    }
}

fn run() -> Result<()> {
    let args = Cli::parse();

    if !args.input.exists() {
        return Err(Error::InputNotFound(args.input.clone()));
    }

    if args.max_size_mb <= 0.0 {
        return Err(Error::InvalidInput("--max-size-mb must be positive".into()));
    }

    if args.chunk_secs <= 0.0 {
        return Err(Error::InvalidInput("--chunk-secs must be positive".into()));
    }

    let info = probe::probe(&args.input)?;
    eprintln!(
        "input: {}x{}, {:.1}fps, {:.1}s",
        info.width, info.height, info.framerate, info.duration_secs
    );

    let target_bytes = (args.max_size_mb * 1024.0 * 1024.0) as u64;
    let output_stem = output_stem_from_args(&args.input, args.output.as_deref())?;
    let chunk_count = (info.duration_secs / args.chunk_secs).ceil() as u32;

    if chunk_count == 0 {
        return Err(Error::InvalidInput("video has zero duration".into()));
    }

    let initial = InitialParams {
        width: args.width.min(info.width),
        fps: args.fps.min(info.framerate.ceil() as u32),
        colors: args.colors,
    };

    let mut outputs: Vec<PathBuf> = Vec::new();

    for chunk_index in 0..chunk_count {
        let start_secs = chunk_index as f64 * args.chunk_secs;
        let remaining = info.duration_secs - start_secs;
        let duration_secs = remaining.min(args.chunk_secs);

        if duration_secs <= 0.0 {
            break;
        }

        let output_path = chunk_output_path(&output_stem, chunk_index, chunk_count);

        eprintln!(
            "\nchunk {}/{}: {:.1}s - {:.1}s -> {}",
            chunk_index + 1,
            chunk_count,
            start_secs,
            start_secs + duration_secs,
            output_path.display()
        );

        let size = strategy::auto_encode(
            &args.input,
            &output_path,
            target_bytes,
            &initial,
            start_secs,
            duration_secs,
        )?;

        let size_mb = size as f64 / (1024.0 * 1024.0);
        eprintln!("  -> {:.2} MB", size_mb);
        outputs.push(output_path);
    }

    eprintln!("\ndone — {} chunk(s) written:", outputs.len());
    for path in &outputs {
        eprintln!("  {}", path.display());
    }

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_chunk_produces_plain_gif_extension() {
        let result = chunk_output_path(Path::new("demo"), 0, 1);
        assert_eq!(result, PathBuf::from("demo.gif"));
    }

    #[test]
    fn multi_chunk_produces_numbered_suffixes() {
        let result = chunk_output_path(Path::new("demo"), 0, 4);
        assert_eq!(result, PathBuf::from("demo_001.gif"));

        let result = chunk_output_path(Path::new("demo"), 3, 4);
        assert_eq!(result, PathBuf::from("demo_004.gif"));
    }

    #[test]
    fn chunk_path_preserves_parent_directory() {
        let stem = Path::new("/tmp/output/demo");
        let result = chunk_output_path(stem, 0, 3);
        assert_eq!(result, PathBuf::from("/tmp/output/demo_001.gif"));
    }

    #[test]
    fn output_stem_strips_extension_from_input() {
        let result = output_stem_from_args(Path::new("video.mp4"), None).unwrap();
        assert_eq!(result, PathBuf::from("video"));
    }

    #[test]
    fn output_stem_uses_explicit_output_without_extension() {
        let result =
            output_stem_from_args(Path::new("video.mp4"), Some(Path::new("out.gif"))).unwrap();
        assert_eq!(result, PathBuf::from("out"));
    }

    #[test]
    fn output_stem_explicit_output_no_extension() {
        let result =
            output_stem_from_args(Path::new("video.mp4"), Some(Path::new("myoutput"))).unwrap();
        assert_eq!(result, PathBuf::from("myoutput"));
    }
}
