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
                    if ui.button("Open").clicked() {}

                    if ui.button("Save").clicked() {}

                    ui.separator();

                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.heading("memflow mirror");
            ui.separator();

            egui::warn_if_debug_build(ui);

            self.frame_history.ui(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
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

        ctx.request_repaint();
    }
}
