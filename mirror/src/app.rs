use ::std::io::Cursor;

use epaint::{Color32, Rect, TextureHandle};

use egui::pos2;
use egui_notify::Toasts;

mod frame_history;
use frame_history::FrameHistory;

use crate::capture_reader::{Capture, ThreadedCapture};
use crate::SequentialCapture;

pub struct MirrorApp {
    _toasts: Toasts,
    frame_history: FrameHistory,

    capture: Box<dyn Capture>,

    frame_counter: u32,
    frame_texture: Option<TextureHandle>,
    cursor: Option<TextureHandle>,

    window_stats: bool,
    window_settings: bool,
}

impl MirrorApp {
    pub fn new(_: &eframe::CreationContext<'_>, capture: Box<dyn Capture>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Self {
            _toasts: Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
            frame_history: FrameHistory::default(),

            capture,

            frame_counter: 0,
            frame_texture: None,
            cursor: None,

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
                // update internal state, then read frame_counter and image_data
                self.capture.update();

                let frame_counter = self.capture.frame_counter();

                // only update frame_texture on demand
                if frame_counter != self.frame_counter {
                    let frame = self.capture.image_data();
                    self.frame_counter = frame_counter;

                    if let Some(frame_texture) = &mut self.frame_texture {
                        frame_texture.set(frame, egui::TextureOptions::LINEAR);
                    } else {
                        self.frame_texture = Some(ui.ctx().load_texture(
                            "frame",
                            frame,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }

                // render frame_texture
                if let Some(frame_texture) = &self.frame_texture {
                    let texture_size = frame_texture.size();
                    let aspect_ratio = texture_size[0] as f32 / texture_size[1] as f32;
                    let desired_height = ui.available_height();
                    let desired_width = desired_height * aspect_ratio;

                    let render_position = ui
                        .add(egui::Image::new(
                            frame_texture.id(),
                            [desired_width, desired_height],
                        ))
                        .rect;

                    // render cursor on top of frame
                    let cursor_data = self.capture.cursor_data();
                    if cursor_data.is_visible != 0 {
                        let cursor = self.cursor_texture(ui);

                        let (x, y, w, h) = {
                            let scale_x = desired_width / texture_size[0] as f32;
                            let scale_y = desired_height / texture_size[1] as f32;
                            (
                                render_position.left() + cursor_data.x as f32 * scale_x,
                                render_position.top() + cursor_data.y as f32 * scale_y,
                                cursor.size()[0] as f32 * scale_x,
                                cursor.size()[1] as f32 * scale_y,
                            )
                        };
                        ui.painter().image(
                            cursor.id(),
                            Rect::from_min_max(pos2(x, y), pos2(x + w, y + h)),
                            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
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
                    let mut multithreading = self.capture.multithreading();
                    if ui
                        .checkbox(&mut multithreading, "Multithreaded Capture")
                        .changed()
                    {
                        let os = self.capture.os();

                        // backup configuration
                        let enable_obs = self.capture.obs_capture();

                        // re-create capture
                        if multithreading {
                            self.capture = Box::new(ThreadedCapture::new(os));
                        } else {
                            self.capture = Box::new(SequentialCapture::new(os));
                        }

                        // reapply configuration
                        self.capture.set_obs_capture(enable_obs);
                    }

                    let mut enable_obs = self.capture.obs_capture();
                    if ui
                        .checkbox(
                            &mut enable_obs,
                            "Enable OBS Capture (when a Fullscreen App is running)",
                        )
                        .changed()
                    {
                        self.capture.set_obs_capture(enable_obs);
                    };
                });
            self.window_settings = window_settings;
        }

        ctx.request_repaint();
    }
}

impl MirrorApp {
    fn cursor_texture<'a>(&mut self, ui: &'a mut egui::Ui) -> &egui::TextureHandle {
        self.cursor.get_or_insert_with(|| {
            // Load the texture only once.
            let image = image::load(
                Cursor::new(&include_bytes!("../resources/cursor.png")[..]),
                image::ImageFormat::Png,
            )
            .unwrap();
            let size = [image.width() as _, image.height() as _];
            let image_buffer = image.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let cursor_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            ui.ctx()
                .load_texture("cursor", cursor_image, Default::default())
        })
    }
}
