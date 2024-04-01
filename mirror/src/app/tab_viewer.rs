use ::std::io::Cursor;

use ::egui_dock::egui::{self, pos2};
use ::egui_dock::NodeIndex;
use ::egui_dock::SurfaceIndex;
use ::epaint::{Color32, Rect, TextureHandle};

use ::memflow::prelude::v1::*;

use crate::{
    capture::{Capture, ThreadedCapture},
    MirrorConfig, SequentialCapture,
};

pub struct TabViewer<'a> {
    pub(crate) added_nodes: &'a mut Vec<(SurfaceIndex, NodeIndex)>,
    pub(crate) config: &'a mut MirrorConfig,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = CaptureTab;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        match &mut tab.capture {
            Some(_) => {
                tab.ui_capturing(ui, self.config);
            }
            None => {
                tab.ui_connection(ui, self.config);
            }
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match &tab.capture {
            Some(_) => format!(
                "Connection #{} ({})",
                tab.id + 1,
                tab.selected_connector.as_ref().unwrap()
            )
            .into(),
            None => format!("Connection #{}", tab.id + 1).into(),
        }
    }

    fn on_add(&mut self, surface: SurfaceIndex, node: NodeIndex) {
        self.added_nodes.push((surface, node));
    }
}

pub struct CaptureTab {
    id: usize,

    // memflow select ui
    inventory: Inventory,
    selected_connector: Option<String>,
    connect_on_startup: bool,

    // capturing
    capture: Option<Box<dyn Capture>>,

    frame_counter: u32,
    frame_texture: Option<TextureHandle>,
    cursor: Option<TextureHandle>,
}

impl CaptureTab {
    pub fn new(id: usize, config: &MirrorConfig) -> Self {
        Self {
            id,

            inventory: Inventory::scan(),
            selected_connector: None,
            connect_on_startup: config.connect_on_startup,

            capture: None,

            frame_counter: 0,
            frame_texture: None,
            cursor: None,
        }
    }

    pub fn connect(id: usize, config: &MirrorConfig) -> Result<Self> {
        if !config.connect_on_startup || config.last_connector.is_none() || config.last_os.is_none()
        {
            return Err(Error(ErrorOrigin::Other, ErrorKind::Configuration));
        }

        let inventory = Inventory::scan();

        let last_connector = config.last_connector.as_ref().unwrap();
        let connector_args = config
            .last_connector_args
            .as_ref()
            .and_then(|s| str::parse(s).ok());
        let connector =
            inventory.create_connector(last_connector, None, connector_args.as_ref())?;

        let last_os = config.last_os.as_ref().unwrap();
        let os_args = config
            .last_os_args
            .as_ref()
            .and_then(|s| str::parse(s).ok());
        let os = inventory.create_os(last_os, Some(connector), os_args.as_ref())?;

        // create capture instance
        let mut capture: Box<dyn Capture> = if config.multithreading {
            Box::new(ThreadedCapture::new(os))
        } else {
            Box::new(SequentialCapture::new(os))
        };
        Self::update_capture_flags(&mut capture, config);

        Ok(Self {
            id,

            inventory,
            selected_connector: Some(last_connector.clone()),
            connect_on_startup: true,

            capture: Some(capture),

            frame_counter: 0,
            frame_texture: None,
            cursor: None,
        })
    }
}

impl CaptureTab {
    fn ui_connection(&mut self, ui: &mut egui::Ui, config: &mut MirrorConfig) {
        ui.label("Connection:".to_string());

        let connectors = self.inventory.available_connectors();
        let mut selected = self
            .selected_connector
            .clone()
            .unwrap_or_else(|| "None".to_string());
        let prev_selected = selected.clone();

        egui::ComboBox::from_label("Connector")
            .selected_text(selected.clone())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut selected, "None".to_string(), "None");
                for option in connectors.iter() {
                    ui.selectable_value(&mut selected, option.clone(), option);
                }
            });

        if selected != prev_selected {
            self.selected_connector = Some(selected);
        }

        ui.add(egui::Checkbox::new(
            &mut self.connect_on_startup,
            "Connect on next startup",
        ));

        // TODO: chaining + os selector
        // TODO: connector+os args
        // TODO: logging window
        // TODO: store last connection + autoconnect
        if ui
            .add_enabled(
                self.selected_connector.is_some(),
                egui::Button::new("Connect"),
            )
            .clicked()
        {
            let connector = self
                .inventory
                .create_connector(self.selected_connector.as_ref().unwrap(), None, None)
                .unwrap(); // TODO:
            let os = self
                .inventory
                .create_os("win32", Some(connector), None)
                .unwrap(); // TODO:

            // create capture instance
            let mut capture: Box<dyn Capture> = if config.multithreading {
                Box::new(ThreadedCapture::new(os))
            } else {
                Box::new(SequentialCapture::new(os))
            };
            Self::update_capture_flags(&mut capture, config);
            self.capture = Some(capture);

            // update configuration
            config.connect_on_startup = self.connect_on_startup;
            config.last_connector = Some(self.selected_connector.as_ref().unwrap().clone());
            config.last_connector_args = None;
            config.last_os = Some("win32".to_string());
            config.last_os_args = None;
            config.save().ok();
        }
    }

    fn ui_capturing(&mut self, ui: &mut egui::Ui, config: &MirrorConfig) {
        ui.vertical_centered(|ui| {
            self.update_capture_config(config);

            let capture = self.capture.as_mut().unwrap();

            // update internal state, then read frame_counter and image_data
            capture.update();

            let frame_counter = capture.frame_counter();

            // only update frame_texture on demand
            if frame_counter != self.frame_counter {
                let frame = capture.image_data();
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
                    .add(egui::Image::new(egui::load::SizedTexture::new(
                        frame_texture.id(),
                        [desired_width, desired_height],
                    )))
                    .rect;

                // render cursor on top of frame
                let cursor_data = capture.cursor_data();
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
    }

    fn update_capture_config(&mut self, config: &MirrorConfig) {
        // update multithreading:
        if let Some(capture) = &mut self.capture {
            if capture.multithreading() != config.multithreading {
                // re-create capture
                let os = capture.os();
                if config.multithreading {
                    self.capture = Some(Box::new(ThreadedCapture::new(os)));
                } else {
                    self.capture = Some(Box::new(SequentialCapture::new(os)));
                }
            }
        }

        // update flags
        if let Some(capture) = &mut self.capture {
            Self::update_capture_flags(capture, config);
        }
    }

    fn update_capture_flags(capture: &mut Box<dyn Capture>, config: &MirrorConfig) {
        capture.set_obs_capture(config.obs_capture);
    }

    fn cursor_texture<'a>(&mut self, ui: &'a mut egui::Ui) -> &egui::TextureHandle {
        self.cursor.get_or_insert_with(|| {
            // Load the texture only once.
            let image = image::load(
                Cursor::new(&include_bytes!("../../resources/cursor.png")[..]),
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
