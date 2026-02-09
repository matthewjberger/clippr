<h1 align="center">clippr</h1>

<p align="center">
  <a href="https://github.com/matthewjberger/clippr"><img alt="github" src="https://img.shields.io/badge/github-matthewjberger/clippr-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20"></a>
  <a href="https://crates.io/crates/clippr"><img alt="crates.io" src="https://img.shields.io/crates/v/clippr.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20"></a>
  <a href="https://github.com/matthewjberger/clippr/blob/main/LICENSE-MIT"><img alt="license" src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=for-the-badge&labelColor=555555" height="20"></a>
</p>

<p align="center"><strong>Convert screen recordings to GitHub-friendly GIFs in one command.</strong></p>

<p align="center">
  <code>cargo install clippr</code>
</p>

clippr converts MP4 screen recordings into chunked GIFs that fit under GitHub's file size limits. It automatically splits long videos into 3-second segments and iteratively compresses each chunk to stay under the target size.

## Prerequisites

[ffmpeg](https://ffmpeg.org/) and ffprobe must be installed and available in your PATH.

## Quick Start

```bash
clippr demo.mp4
```

This produces `demo_001.gif`, `demo_002.gif`, etc. — one per 3-second chunk, each under 10MB.

## Usage

```bash
clippr demo.mp4                         # Outputs chunked GIFs, auto-sized to <10MB each
clippr demo.mp4 -o out.gif              # Custom output base name
clippr demo.mp4 --max-size-mb 5         # Custom size target per chunk
clippr demo.mp4 --width 640 --fps 20    # Override starting parameters
clippr demo.mp4 --chunk-secs 5          # 5-second chunks instead of 3
```

If the video is 3 seconds or shorter, a single `demo.gif` is produced without a numeric suffix.

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `-o, --output` | Input filename | Output base path (extension stripped, chunks get `_001`, `_002`, etc.) |
| `--max-size-mb` | `10` | Maximum file size per chunk in MB |
| `--width` | `480` | Starting width in pixels (capped at source width) |
| `--fps` | `15` | Starting frames per second (capped at source framerate) |
| `--colors` | `256` | Starting palette color count |
| `--chunk-secs` | `3.0` | Duration of each chunk in seconds |

## How It Works

1. **Probe** the input video with ffprobe to get dimensions, framerate, and duration
2. **Split** the video into chunks of `--chunk-secs` duration
3. **Encode** each chunk as a GIF using ffmpeg's single-pass palettegen/paletteuse pipeline with lanczos scaling and Floyd-Steinberg dithering
4. **Auto-size** — if a chunk exceeds the target size, iteratively reduce parameters and re-encode:
   - **Width** first (sqrt of ratio, since pixel count is quadratic) — minimum 240px
   - **FPS** second — minimum 8fps
   - **Colors** last (256 → 128 → 64 → 32) — minimum 32
5. Fail with a clear error if the target is unreachable after 5 attempts

The source video is never modified.

## Example Output

```
$ clippr recording.mp4
input: 802x632, 30.0fps, 10.9s

chunk 1/4: 0.0s - 3.0s -> recording_001.gif
  attempt 1: 480px, 15fps, 256 colors
  -> 4.75 MB

chunk 2/4: 3.0s - 6.0s -> recording_002.gif
  attempt 1: 480px, 15fps, 256 colors
  -> 5.03 MB

chunk 3/4: 6.0s - 9.0s -> recording_003.gif
  attempt 1: 480px, 15fps, 256 colors
  -> 3.04 MB

chunk 4/4: 9.0s - 10.9s -> recording_004.gif
  attempt 1: 480px, 15fps, 256 colors
  -> 1.12 MB

done — 4 chunk(s) written:
  recording_001.gif
  recording_002.gif
  recording_003.gif
  recording_004.gif
```

## License

Dual-licensed under MIT ([LICENSE-MIT](LICENSE-MIT)) or Apache 2.0 ([LICENSE-APACHE](LICENSE-APACHE)).
