use crate::encode::{self, EncodeParams};
use crate::error::{Error, Result};
use std::path::Path;

const MAX_ATTEMPTS: u32 = 5;
const MIN_WIDTH: u32 = 240;
const MIN_FPS: u32 = 8;
const COLOR_STEPS: &[u32] = &[256, 128, 64, 32];
const SAFETY_MARGIN: f64 = 0.90;

pub struct InitialParams {
    pub width: u32,
    pub fps: u32,
    pub colors: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EncodeSettings {
    width: u32,
    fps: u32,
    color_index: usize,
}

fn resolve_color_index(requested_colors: u32) -> usize {
    COLOR_STEPS
        .iter()
        .position(|&color| color <= requested_colors)
        .unwrap_or(0)
}

fn reduce_params(settings: &EncodeSettings, ratio: f64) -> Option<EncodeSettings> {
    let new_width = ((settings.width as f64) * ratio.sqrt()).max(MIN_WIDTH as f64) as u32;
    if new_width < settings.width {
        return Some(EncodeSettings {
            width: new_width,
            fps: settings.fps,
            color_index: settings.color_index,
        });
    }

    let new_fps = ((settings.fps as f64) * ratio).max(MIN_FPS as f64) as u32;
    if new_fps < settings.fps {
        return Some(EncodeSettings {
            width: settings.width,
            fps: new_fps,
            color_index: settings.color_index,
        });
    }

    if settings.color_index + 1 < COLOR_STEPS.len() {
        return Some(EncodeSettings {
            width: settings.width,
            fps: settings.fps,
            color_index: settings.color_index + 1,
        });
    }

    None
}

pub fn auto_encode(
    input: &Path,
    output: &Path,
    target_bytes: u64,
    initial: &InitialParams,
    start_secs: f64,
    duration_secs: f64,
) -> Result<u64> {
    let mut settings = EncodeSettings {
        width: initial.width,
        fps: initial.fps,
        color_index: resolve_color_index(initial.colors),
    };

    for attempt in 0..MAX_ATTEMPTS {
        let params = EncodeParams {
            width: settings.width,
            fps: settings.fps,
            colors: COLOR_STEPS[settings.color_index],
            start_secs,
            duration_secs,
        };

        eprintln!(
            "  attempt {}: {}px, {}fps, {} colors",
            attempt + 1,
            settings.width,
            settings.fps,
            COLOR_STEPS[settings.color_index]
        );

        let size = encode::encode(input, output, &params)?;

        if size <= target_bytes {
            return Ok(size);
        }

        let ratio = (target_bytes as f64 / size as f64) * SAFETY_MARGIN;

        settings = match reduce_params(&settings, ratio) {
            Some(reduced) => reduced,
            None => return Err(Error::TargetUnreachable(attempt + 1)),
        };
    }

    Err(Error::TargetUnreachable(MAX_ATTEMPTS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_prefers_width_first() {
        let settings = EncodeSettings {
            width: 480,
            fps: 15,
            color_index: 0,
        };
        let result = reduce_params(&settings, 0.5).unwrap();
        assert!(result.width < 480);
        assert_eq!(result.fps, 15);
        assert_eq!(result.color_index, 0);
    }

    #[test]
    fn reduce_falls_through_to_fps_when_width_floored() {
        let settings = EncodeSettings {
            width: MIN_WIDTH,
            fps: 15,
            color_index: 0,
        };
        let result = reduce_params(&settings, 0.5).unwrap();
        assert_eq!(result.width, MIN_WIDTH);
        assert!(result.fps < 15);
        assert_eq!(result.color_index, 0);
    }

    #[test]
    fn reduce_falls_through_to_colors_when_width_and_fps_floored() {
        let settings = EncodeSettings {
            width: MIN_WIDTH,
            fps: MIN_FPS,
            color_index: 0,
        };
        let result = reduce_params(&settings, 0.5).unwrap();
        assert_eq!(result.width, MIN_WIDTH);
        assert_eq!(result.fps, MIN_FPS);
        assert_eq!(result.color_index, 1);
    }

    #[test]
    fn reduce_returns_none_when_all_floored() {
        let settings = EncodeSettings {
            width: MIN_WIDTH,
            fps: MIN_FPS,
            color_index: COLOR_STEPS.len() - 1,
        };
        assert!(reduce_params(&settings, 0.5).is_none());
    }

    #[test]
    fn reduce_clamps_width_to_minimum() {
        let settings = EncodeSettings {
            width: 260,
            fps: 15,
            color_index: 0,
        };
        let result = reduce_params(&settings, 0.01).unwrap();
        assert_eq!(result.width, MIN_WIDTH);
    }

    #[test]
    fn reduce_clamps_fps_to_minimum() {
        let settings = EncodeSettings {
            width: MIN_WIDTH,
            fps: 15,
            color_index: 0,
        };
        let result = reduce_params(&settings, 0.01).unwrap();
        assert_eq!(result.fps, MIN_FPS);
    }

    #[test]
    fn reduce_width_uses_sqrt_of_ratio() {
        let settings = EncodeSettings {
            width: 480,
            fps: 15,
            color_index: 0,
        };
        let ratio = 0.5;
        let result = reduce_params(&settings, ratio).unwrap();
        let expected = ((480.0_f64) * ratio.sqrt()).max(MIN_WIDTH as f64) as u32;
        assert_eq!(result.width, expected);
    }

    #[test]
    fn reduce_fps_uses_linear_ratio() {
        let settings = EncodeSettings {
            width: MIN_WIDTH,
            fps: 15,
            color_index: 0,
        };
        let ratio = 0.7;
        let result = reduce_params(&settings, ratio).unwrap();
        let expected = ((15.0_f64) * ratio).max(MIN_FPS as f64) as u32;
        assert_eq!(result.fps, expected);
    }

    #[test]
    fn reduce_steps_colors_one_at_a_time() {
        let settings = EncodeSettings {
            width: MIN_WIDTH,
            fps: MIN_FPS,
            color_index: 1,
        };
        let result = reduce_params(&settings, 0.5).unwrap();
        assert_eq!(result.color_index, 2);
    }

    #[test]
    fn resolve_color_index_exact_match() {
        assert_eq!(resolve_color_index(256), 0);
        assert_eq!(resolve_color_index(128), 1);
        assert_eq!(resolve_color_index(64), 2);
        assert_eq!(resolve_color_index(32), 3);
    }

    #[test]
    fn resolve_color_index_rounds_down_to_nearest_step() {
        assert_eq!(resolve_color_index(200), 1);
        assert_eq!(resolve_color_index(100), 2);
        assert_eq!(resolve_color_index(50), 3);
        assert_eq!(resolve_color_index(33), 3);
    }

    #[test]
    fn resolve_color_index_above_max_gives_first() {
        assert_eq!(resolve_color_index(512), 0);
    }

    #[test]
    fn reduce_only_changes_one_param_per_call() {
        let settings = EncodeSettings {
            width: 480,
            fps: 15,
            color_index: 0,
        };
        let result = reduce_params(&settings, 0.3).unwrap();
        let mut changes = 0;
        if result.width != settings.width {
            changes += 1;
        }
        if result.fps != settings.fps {
            changes += 1;
        }
        if result.color_index != settings.color_index {
            changes += 1;
        }
        assert_eq!(changes, 1);
    }
}
