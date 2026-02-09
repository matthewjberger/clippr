use nightshade::prelude::*;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    launch(ClipprUi::default())
}

#[derive(Default, PartialEq)]
enum ConversionStatus {
    #[default]
    Idle,
    Running,
    Done,
    Failed(String),
}

struct ClipprUi {
    input_path: Option<PathBuf>,
    output_path: String,
    max_size_mb: f64,
    width: u32,
    fps: u32,
    colors: u32,
    chunk_secs: f64,
    log_lines: Vec<String>,
    status: ConversionStatus,
    log_receiver: Option<mpsc::Receiver<LogMessage>>,
}

enum LogMessage {
    Line(String),
    Finished { success: bool, message: String },
}

impl Default for ClipprUi {
    fn default() -> Self {
        Self {
            input_path: None,
            output_path: String::new(),
            max_size_mb: 10.0,
            width: 480,
            fps: 15,
            colors: 256,
            chunk_secs: 3.0,
            log_lines: Vec::new(),
            status: ConversionStatus::Idle,
            log_receiver: None,
        }
    }
}

impl ClipprUi {
    fn start_conversion(&mut self) {
        let input_path = match &self.input_path {
            Some(path) => path.clone(),
            None => return,
        };

        self.log_lines.clear();
        self.status = ConversionStatus::Running;

        let mut args: Vec<String> = vec![input_path.to_string_lossy().into_owned()];

        args.push("--max-size-mb".into());
        args.push(format!("{}", self.max_size_mb));

        args.push("--width".into());
        args.push(format!("{}", self.width));

        args.push("--fps".into());
        args.push(format!("{}", self.fps));

        args.push("--colors".into());
        args.push(format!("{}", self.colors));

        args.push("--chunk-secs".into());
        args.push(format!("{}", self.chunk_secs));

        if !self.output_path.is_empty() {
            args.push("--output".into());
            args.push(self.output_path.clone());
        }

        let (sender, receiver) = mpsc::channel();
        self.log_receiver = Some(receiver);

        std::thread::spawn(move || {
            let result = Command::new("clippr")
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();

            let mut child = match result {
                Ok(child) => child,
                Err(error) => {
                    let _ = sender.send(LogMessage::Finished {
                        success: false,
                        message: format!("failed to start clippr: {error}"),
                    });
                    return;
                }
            };

            if let Some(stderr) = child.stderr.take() {
                let reader = std::io::BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(text) => {
                            if sender.send(LogMessage::Line(text)).is_err() {
                                return;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }

            match child.wait() {
                Ok(exit_status) => {
                    if exit_status.success() {
                        let _ = sender.send(LogMessage::Finished {
                            success: true,
                            message: "conversion complete".into(),
                        });
                    } else {
                        let code = exit_status
                            .code()
                            .map(|code| format!("exit code {code}"))
                            .unwrap_or_else(|| "unknown exit status".into());
                        let _ = sender.send(LogMessage::Finished {
                            success: false,
                            message: format!("clippr failed: {code}"),
                        });
                    }
                }
                Err(error) => {
                    let _ = sender.send(LogMessage::Finished {
                        success: false,
                        message: format!("failed to wait for clippr: {error}"),
                    });
                }
            }
        });
    }

    fn drain_log_messages(&mut self) {
        let receiver = match &self.log_receiver {
            Some(receiver) => receiver,
            None => return,
        };

        loop {
            match receiver.try_recv() {
                Ok(LogMessage::Line(text)) => {
                    self.log_lines.push(text);
                }
                Ok(LogMessage::Finished { success, message }) => {
                    self.log_lines.push(message.clone());
                    if success {
                        self.status = ConversionStatus::Done;
                    } else {
                        self.status = ConversionStatus::Failed(message);
                    }
                    self.log_receiver = None;
                    return;
                }
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.status =
                        ConversionStatus::Failed("lost connection to clippr process".into());
                    self.log_receiver = None;
                    return;
                }
            }
        }
    }
}

impl State for ClipprUi {
    fn title(&self) -> &str {
        "clippr"
    }

    fn initialize(&mut self, world: &mut World) {
        world.resources.user_interface.enabled = true;
        world.resources.graphics.show_grid = false;
        world.resources.graphics.atmosphere = Atmosphere::None;

        let camera_entity = spawn_pan_orbit_camera(
            world,
            Vec3::new(0.0, 0.0, 0.0),
            10.0,
            0.0,
            0.0,
            "Main Camera".to_string(),
        );
        world.resources.active_camera = Some(camera_entity);
    }

    fn ui(&mut self, _world: &mut World, ui_context: &egui::Context) {
        egui::CentralPanel::default().show(ui_context, |ui| {
            ui.heading("clippr");
            ui.separator();

            ui.horizontal(|ui| {
                let label = match &self.input_path {
                    Some(path) => path.to_string_lossy().to_string(),
                    None => "No file selected".to_string(),
                };
                ui.label(&label);
                if ui.button("Browse...").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Video", &["mp4", "mkv", "avi", "mov", "webm"])
                        .pick_file()
                {
                    self.input_path = Some(path);
                }
            });

            ui.separator();
            ui.label("Parameters");

            egui::Grid::new("params_grid")
                .num_columns(2)
                .spacing([20.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Max size (MB):");
                    ui.add(
                        egui::DragValue::new(&mut self.max_size_mb)
                            .range(0.1..=100.0)
                            .speed(0.1),
                    );
                    ui.end_row();

                    ui.label("Width (px):");
                    ui.add(egui::DragValue::new(&mut self.width).range(100..=3840));
                    ui.end_row();

                    ui.label("FPS:");
                    ui.add(egui::DragValue::new(&mut self.fps).range(1..=60));
                    ui.end_row();

                    ui.label("Colors:");
                    ui.add(egui::DragValue::new(&mut self.colors).range(2..=256));
                    ui.end_row();

                    ui.label("Chunk duration (s):");
                    ui.add(
                        egui::DragValue::new(&mut self.chunk_secs)
                            .range(0.5..=30.0)
                            .speed(0.1),
                    );
                    ui.end_row();
                });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Output path:");
                ui.text_edit_singleline(&mut self.output_path);
                if ui.button("Browse...").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("GIF", &["gif"])
                        .save_file()
                {
                    self.output_path = path.to_string_lossy().into_owned();
                }
            });

            ui.separator();

            let can_convert = self.input_path.is_some() && self.status != ConversionStatus::Running;

            ui.add_enabled_ui(can_convert, |ui| {
                if ui.button("Convert").clicked() {
                    self.start_conversion();
                }
            });

            ui.separator();

            let status_text = match &self.status {
                ConversionStatus::Idle => "Idle",
                ConversionStatus::Running => "Running...",
                ConversionStatus::Done => "Done",
                ConversionStatus::Failed(_) => "Failed",
            };
            ui.label(format!("Status: {status_text}"));

            if let ConversionStatus::Failed(message) = &self.status {
                ui.colored_label(egui::Color32::RED, message);
            }

            ui.separator();
            ui.label("Log");

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.log_lines {
                        ui.monospace(line);
                    }
                });
        });
    }

    fn run_systems(&mut self, _world: &mut World) {
        self.drain_log_messages();
    }

    fn on_keyboard_input(&mut self, world: &mut World, key_code: KeyCode, key_state: KeyState) {
        if matches!((key_code, key_state), (KeyCode::KeyQ, KeyState::Pressed))
            && self.status != ConversionStatus::Running
        {
            world.resources.window.should_exit = true;
        }
    }
}
