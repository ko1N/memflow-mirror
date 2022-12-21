use std::io::Write;
use std::{fs::File, time::Duration};

use epaint::TextureHandle;
use log::{error, info};

use egui::{pos2, ScrollArea};
use egui_notify::Toasts;

mod frame_history;
use frame_history::FrameHistory;

use memflow::prelude::v1::*;

use crate::CaptureReader;

pub struct MirrorApp {
    toasts: Toasts,
    frame_history: FrameHistory,

    reader: CaptureReader,

    texture: Option<TextureHandle>,

    window_stats: bool,
    window_settings: bool,
}

impl MirrorApp {
    pub fn new(_: &eframe::CreationContext<'_>, reader: CaptureReader) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Self {
            toasts: Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
            frame_history: FrameHistory::default(),

            reader,

            texture: None,

            window_stats: false,
            window_settings: false,
        }
    }
}

impl eframe::App for MirrorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.frame_history
            .on_new_frame(ctx.input().time, frame.info().cpu_usage);

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
                ui.menu_button("Windows", |ui| {
                    if ui.button("Stats").clicked() {
                        self.window_stats = !self.window_stats;
                        ui.close_menu();
                    }

                    if ui.button("Settings").clicked() {
                        self.window_settings = !self.window_settings;
                        ui.close_menu();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::warn_if_debug_build(ui);

            ui.vertical_centered(|ui| {
                // TODO: check if new frame?
                let frame = self.reader.image_data();

                let aspect_ratio = frame.width() as f32 / frame.height() as f32;
                let desired_height = ui.available_height();
                let desired_width = desired_height * aspect_ratio;

                if let Some(texture) = &mut self.texture {
                    texture.set(frame, egui::TextureOptions::LINEAR);
                } else {
                    self.texture = Some(ui.ctx().load_texture(
                        "frame",
                        frame,
                        egui::TextureOptions::LINEAR,
                    ));
                }

                if let Some(texture) = &self.texture {
                    ui.add(egui::Image::new(
                        texture.id(),
                        [desired_width, desired_height],
                    ));
                }
            });
        });

        // windows
        if self.window_stats {
            let mut window_stats = true;
            egui::Window::new("Stats")
                .collapsible(false)
                .open(&mut window_stats)
                .show(ctx, |ui| {
                    self.frame_history.ui(ui);
                });
            self.window_stats = window_stats;
        }

        if self.window_settings {
            let mut window_settings = true;
            egui::Window::new("Settings")
                .collapsible(false)
                .open(&mut window_settings)
                .show(ctx, |ui| {
                    let mut multithreading = self.reader.multithreading();
                    ui.checkbox(&mut multithreading, "Multithreading");
                    if multithreading != self.reader.multithreading() {
                        let os = self.reader.os();
                        self.reader = CaptureReader::new(os, multithreading);
                    }

                    let mut enable_dxgi = true;
                    ui.checkbox(&mut enable_dxgi, "Enable DXGI Capture (if available)");

                    let mut enable_obs = true;
                    ui.checkbox(
                        &mut enable_obs,
                        "Enable OBS Capture (when a Fullscreen App is running)",
                    );
                });
            self.window_settings = window_settings;
        }

        ctx.request_repaint();
    }
}