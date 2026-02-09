pub mod encode;
pub mod error;
#[cfg(feature = "gui")]
pub mod gui;
pub mod probe;
pub mod strategy;

use encode::EncodeParams;
use error::{Error, Result};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use strategy::InitialParams;

const MIN_SPLIT_DURATION: f64 = 0.5;

pub struct ConvertOptions {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub max_size_mb: f64,
    pub width: u32,
    pub fps: u32,
    pub colors: u32,
    pub chunk_secs: f64,
}

#[derive(Clone)]
struct Segment {
    start_secs: f64,
    duration_secs: f64,
}

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

fn temp_output_path(stem: &Path, index: u32) -> PathBuf {
    let name = format!(
        "{}.tmp_{:06}.gif",
        stem.file_name().unwrap_or_default().to_string_lossy(),
        index,
    );
    stem.with_file_name(name)
}

pub fn convert(
    options: &ConvertOptions,
    mut on_progress: impl FnMut(&str),
) -> Result<Vec<PathBuf>> {
    if !options.input.exists() {
        return Err(Error::InputNotFound(options.input.clone()));
    }

    if options.max_size_mb <= 0.0 {
        return Err(Error::InvalidInput("--max-size-mb must be positive".into()));
    }

    if options.chunk_secs <= 0.0 {
        return Err(Error::InvalidInput("--chunk-secs must be positive".into()));
    }

    let info = probe::probe(&options.input)?;
    on_progress(&format!(
        "input: {}x{}, {:.1}fps, {:.1}s",
        info.width, info.height, info.framerate, info.duration_secs
    ));

    let target_bytes = (options.max_size_mb * 1024.0 * 1024.0) as u64;
    let output_stem = output_stem_from_args(&options.input, options.output.as_deref())?;
    let initial_chunk_count = (info.duration_secs / options.chunk_secs).ceil() as u32;

    if initial_chunk_count == 0 {
        return Err(Error::InvalidInput("video has zero duration".into()));
    }

    let initial = InitialParams {
        width: options.width.min(info.width),
        fps: options.fps.min(info.framerate.ceil() as u32),
        colors: options.colors,
    };

    let mut queue: VecDeque<Segment> = VecDeque::new();
    for chunk_index in 0..initial_chunk_count {
        let start_secs = chunk_index as f64 * options.chunk_secs;
        let remaining = info.duration_secs - start_secs;
        let duration_secs = remaining.min(options.chunk_secs);
        if duration_secs > 0.0 {
            queue.push_back(Segment {
                start_secs,
                duration_secs,
            });
        }
    }

    let mut temp_paths: Vec<PathBuf> = Vec::new();
    let mut temp_counter: u32 = 0;

    while let Some(segment) = queue.pop_front() {
        let temp_path = temp_output_path(&output_stem, temp_counter);
        temp_counter += 1;

        on_progress(&format!(
            "\nsegment: {:.1}s - {:.1}s ({:.1}s)",
            segment.start_secs,
            segment.start_secs + segment.duration_secs,
            segment.duration_secs,
        ));

        let params = EncodeParams {
            width: initial.width,
            fps: initial.fps,
            colors: initial.colors,
            start_secs: segment.start_secs,
            duration_secs: segment.duration_secs,
        };

        let size = encode::encode(&options.input, &temp_path, &params)?;

        if size <= target_bytes {
            let size_mb = size as f64 / (1024.0 * 1024.0);
            on_progress(&format!("  -> {:.2} MB (fits at full quality)", size_mb));
            temp_paths.push(temp_path);
            continue;
        }

        std::fs::remove_file(&temp_path)?;

        if segment.duration_secs > MIN_SPLIT_DURATION {
            let half = segment.duration_secs / 2.0;
            on_progress(&format!(
                "  -> {:.2} MB (too large, splitting {:.1}s into 2x {:.1}s)",
                size as f64 / (1024.0 * 1024.0),
                segment.duration_secs,
                half,
            ));
            queue.push_front(Segment {
                start_secs: segment.start_secs + half,
                duration_secs: segment.duration_secs - half,
            });
            queue.push_front(Segment {
                start_secs: segment.start_secs,
                duration_secs: half,
            });
            continue;
        }

        on_progress(&format!(
            "  -> {:.2} MB (too large, segment too short to split — degrading quality)",
            size as f64 / (1024.0 * 1024.0),
        ));

        let temp_path = temp_output_path(&output_stem, temp_counter);
        temp_counter += 1;

        let size = strategy::auto_encode(
            &options.input,
            &temp_path,
            target_bytes,
            &initial,
            segment.start_secs,
            segment.duration_secs,
            &mut on_progress,
        )?;

        let size_mb = size as f64 / (1024.0 * 1024.0);
        on_progress(&format!("  -> {:.2} MB (degraded quality)", size_mb));
        temp_paths.push(temp_path);
    }

    let final_count = temp_paths.len() as u32;
    let mut outputs: Vec<PathBuf> = Vec::new();

    for (index, temp_path) in temp_paths.iter().enumerate() {
        let final_path = chunk_output_path(&output_stem, index as u32, final_count);
        std::fs::rename(temp_path, &final_path)?;
        outputs.push(final_path);
    }

    on_progress(&format!("\ndone — {} chunk(s) written:", outputs.len()));
    for path in &outputs {
        on_progress(&format!("  {}", path.display()));
    }

    Ok(outputs)
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
