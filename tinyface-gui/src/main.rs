//! `tinyface-gui` — Graphical RME interface controller.
//!
//! Uses egui/eframe for a cross-platform GUI that mimics the
//! TotalMix workflow.

use eframe::egui;
use tinyface_core::{BabyfacePro, RmeDevice};

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    // Try to detect the device at startup.
    let device = match BabyfacePro::open() {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("Could not open device: {}", e);
            eprintln!("The UI will run in offline mode.");
            None
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_title("Tinyface — RME Mixer"),
        ..Default::default()
    };

    eframe::run_native(
        "Tinyface",
        options,
        Box::new(|_cc| Ok(Box::new(TinyFaceApp::new(device)))),
    )
}

struct TinyFaceApp {
    device: Option<BabyfacePro>,
    selected_output: usize,
}

impl TinyFaceApp {
    fn new(device: Option<BabyfacePro>) -> Self {
        Self {
            device,
            selected_output: 0,
        }
    }
}

impl eframe::App for TinyFaceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll device events periodically
        if let Some(ref mut device) = self.device {
            let _ = device.poll_events();
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        // ── Top panel ──────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Tinyface");
                match &self.device {
                    Some(d) => {
                        ui.label(format!("— {} ", d.model_name()));
                        ui.colored_label(egui::Color32::GREEN, "● Connected");
                    }
                    None => {
                        ui.colored_label(egui::Color32::GRAY, "● Offline");
                    }
                }
            });
        });

        // ── Center content ─────────────────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref device) = self.device {
                // ── Hardware Inputs ─────────────────────────────
                ui.label("Hardware Inputs");
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for input in device.inputs() {
                            ui.vertical(|ui| {
                                ui.set_min_width(80.0);
                                ui.label(&input.name);
                                ui.label(format!("{:?}", input.channel_type));
                                if input.phantom {
                                    ui.colored_label(egui::Color32::RED, "48V");
                                }
                                if input.pad {
                                    ui.label("PAD");
                                }
                                ui.add(
                                    egui::Slider::new(&mut input.volume.to_owned(), 0.0..=1.0)
                                        .vertical()
                                        .show_value(false),
                                );
                            });
                        }
                    });
                });

                ui.separator();

                // ── Software Playbacks ──────────────────────────
                ui.label("Software Playback");
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for pb in device.playbacks() {
                            ui.vertical(|ui| {
                                ui.set_min_width(80.0);
                                ui.label(&pb.name);
                                ui.add(
                                    egui::Slider::new(&mut pb.volume.to_owned(), 0.0..=1.0)
                                        .vertical()
                                        .show_value(false),
                                );
                            });
                        }
                    });
                });

                ui.separator();

                // ── Output routing selector ─────────────────────
                ui.horizontal(|ui| {
                    ui.label("Hardware Output:");
                    let outputs = [
                        "AN1/AN2",
                        "PH3/PH4",
                        "AS1/AS2",
                        "ADAT3/ADAT4",
                        "ADAT5/ADAT6",
                        "ADAT7/ADAT8",
                    ];
                    egui::ComboBox::from_id_salt("output_selector")
                        .selected_text(outputs[self.selected_output])
                        .show_ui(ui, |ui| {
                            for (i, name) in outputs.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_output, i, *name);
                            }
                        });
                });

                // ── Global settings ─────────────────────────────
                ui.separator();
                ui.label("Settings");
                if let Some(ref device) = self.device {
                    ui.horizontal(|ui| {
                        ui.label(format!("Clock: {}", device.settings().clock_source));
                    });
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.heading("No device detected");
                    ui.label(
                        "Connect your RME Babyface Pro FS and restart the application.\n\
                         Make sure the device is recognised by ALSA:\n\n\
                         $ amixer -c <card> contents",
                    );
                });
            }
        });
    }
}
