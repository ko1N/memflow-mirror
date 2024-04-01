use ::log::warn;

use ::egui_dock::{DockArea, DockState, Style};
use ::egui_notify::Toasts;

mod frame_history;
use frame_history::FrameHistory;

mod tab_viewer;
use tab_viewer::{CaptureTab, TabViewer};

use crate::MirrorConfig;

pub struct MirrorApp {
    _toasts: Toasts,
    frame_history: FrameHistory,
    tree: DockState<CaptureTab>,
    tree_len: usize,

    config: MirrorConfig,

    window_stats: bool,
    window_settings: bool,
}

impl MirrorApp {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let config = MirrorConfig::load_or_default();
        let capture_tab = match CaptureTab::connect(0, &config) {
            Ok(capture_tab) => capture_tab,
            Err(_) => {
                //config.connect_on_startup = false;
                //config.save().ok();
                CaptureTab::new(0, &config)
            }
        };

        Self {
            _toasts: Toasts::default().with_anchor(egui_notify::Anchor::BottomRight),
            frame_history: FrameHistory::default(),
            tree: DockState::new(vec![capture_tab]),
            tree_len: 1,

            config,

            window_stats: false,
            window_settings: false,
        }
    }
}

impl eframe::App for MirrorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.frame_history
            .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);

        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
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

            let mut added_nodes = Vec::new();
            DockArea::new(&mut self.tree)
                .show_add_buttons(true)
                .style({
                    let mut style = Style::from_egui(ctx.style().as_ref());
                    style.tab_bar.fill_tab_bar = true;
                    style
                })
                .show(
                    ctx,
                    &mut TabViewer {
                        added_nodes: &mut added_nodes,
                        config: &mut self.config,
                    },
                );

            added_nodes.drain(..).for_each(|(surface, node)| {
                self.tree.set_focused_node_and_surface((surface, node));
                self.tree
                    .push_to_focused_leaf(CaptureTab::new(self.tree_len, &self.config));
                self.tree_len += 1;
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
                    let mut connect_on_startup = self.config.connect_on_startup;
                    if ui
                        .checkbox(&mut connect_on_startup, "Auto-connect on startup")
                        .changed()
                    {
                        self.config.connect_on_startup = connect_on_startup;
                        self.config.save().map_err(|err| warn!("{}", err)).ok();
                    }

                    ui.separator();

                    let mut multithreading = self.config.multithreading;
                    if ui
                        .checkbox(&mut multithreading, "Multithreaded Capture")
                        .changed()
                    {
                        self.config.multithreading = multithreading;
                        self.config.save().map_err(|err| warn!("{}", err)).ok();
                    }

                    let mut obs_capture = self.config.obs_capture;
                    if ui
                        .checkbox(
                            &mut obs_capture,
                            "Enable OBS Capture (when a Fullscreen App is running)",
                        )
                        .changed()
                    {
                        self.config.obs_capture = obs_capture;
                        self.config.save().map_err(|err| warn!("{}", err)).ok();
                    };
                });
            self.window_settings = window_settings;
        }

        ctx.request_repaint();
    }
}
