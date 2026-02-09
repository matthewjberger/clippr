# clippr-ui

A native GUI frontend for the [clippr](../clippr) CLI tool. Select a video file, configure conversion parameters (chunk duration, size limit, resolution, frame rate, palette size), run the conversion, and watch live progress.

## Prerequisites

- `clippr` CLI must be in your PATH
- `ffmpeg` must be in your PATH

## Usage

From the `clippr-ui` directory:

```bash
just run
```

Or from the workspace root:

```bash
cargo run -r -p clippr-ui
```
