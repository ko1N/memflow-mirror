mod glium_sdl2;
use glium_sdl2::DisplayBuild;

use std::path::Path;

#[allow(unused)]
use log::{info, warn};
#[allow(unused)]
use std::sync::mpsc::channel;
#[allow(unused)]
use std::time::Duration;

use glium::{
    backend::Facade, implement_vertex, texture::SrgbTexture2d, uniform,
    uniforms::MagnifySamplerFilter, Surface,
};
use glium_text_rusttype as glium_text;
use sdl2::{self, video::SwapInterval};

#[cfg(feature = "shader-reload")]
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};

pub struct Window {
    pub display: glium_sdl2::SDL2Facade,
    pub event_pump: sdl2::EventPump,

    pub text_system: glium_text::TextSystem,
    pub font: glium_text::FontTexture, // TODO: map

    pub program: glium::Program, // TODO: map

    #[cfg(feature = "shader-reload")]
    _watcher: notify::INotifyWatcher,
    #[cfg(feature = "shader-reload")]
    resources_update_rx: std::sync::mpsc::Receiver<notify::DebouncedEvent>,
}

// loads a vertex and fragment shader from file and compiles it
#[allow(unused)]
fn load_shader_program<F: Facade + ?Sized, P: AsRef<Path>>(
    facade: &F,
    vertex_file: P,
    fragment_file: P,
) -> Result<glium::Program, glium::ProgramCreationError> {
    let vertex_shader_src = std::fs::read_to_string(vertex_file).map_err(|e| {
        glium::ProgramCreationError::CompilationError(
            format!("unable to read file: {}", e),
            glium::program::ShaderType::Vertex,
        )
    })?;
    let fragment_shader_src = std::fs::read_to_string(fragment_file).map_err(|e| {
        glium::ProgramCreationError::CompilationError(
            format!("unable to read file: {}", e),
            glium::program::ShaderType::Fragment,
        )
    })?;
    glium::Program::from_source(facade, &vertex_shader_src, &fragment_shader_src, None)
}

#[allow(unused)]
impl Window {
    pub fn new(vsync: bool) -> Self {
        let sdl = sdl2::init().unwrap();
        let video_subsystem = sdl.video().unwrap();
        let display = video_subsystem
            .window("Memflow Mirror", 1280, 720)
            .resizable()
            .build_glium()
            .unwrap();

        if !vsync {
            // disable vsync
            video_subsystem.gl_set_swap_interval(SwapInterval::Immediate);
        }

        let event_pump = sdl.event_pump().unwrap();

        let text_system = glium_text::TextSystem::new(&display);
        let font = glium_text::FontTexture::new(
            &display,
            &include_bytes!("../resources/Allerta-Regular.ttf")[..],
            70,
            glium_text::FontTexture::ascii_character_list(),
        )
        .unwrap();

        // setup basic shaders
        #[cfg(not(feature = "shader-reload"))]
        let program = glium::Program::from_source(
            &display,
            &include_str!("../resources/vertex.glsl"),
            &include_str!("../resources/fragment.glsl"),
            None,
        )
        .unwrap();

        #[cfg(feature = "shader-reload")]
        let program = load_shader_program(
            &display,
            "mirror/resources/vertex.glsl",
            "mirror/resources/fragment.glsl",
        )
        .unwrap();

        // register hot reload handler
        #[cfg(feature = "shader-reload")]
        let (watcher, resources_update_rx) = {
            let (resources_update_tx, resources_update_rx) = channel();
            let mut watcher = watcher(resources_update_tx, Duration::from_millis(500)).unwrap();
            watcher
                .watch("mirror/resources", RecursiveMode::NonRecursive)
                .unwrap();
            (watcher, resources_update_rx)
        };

        Self {
            display,
            event_pump,

            text_system,
            font,

            program,

            #[cfg(feature = "shader-reload")]
            _watcher: watcher,
            #[cfg(feature = "shader-reload")]
            resources_update_rx,
        }
    }

    pub fn frame(&mut self) -> WindowFrame {
        // check for file watcher updates
        #[cfg(feature = "shader-reload")]
        if let Ok(DebouncedEvent::Write(file)) = self.resources_update_rx.try_recv() {
            if file.extension().is_some() && file.extension().unwrap() == "glsl" {
                match load_shader_program(
                    &self.display,
                    "mirror/resources/vertex.glsl",
                    "mirror/resources/fragment.glsl",
                ) {
                    Ok(program) => {
                        info!("shader reload successful");
                        self.program = program;
                    }
                    Err(err) => {
                        warn!("failed to reload shader: {}", err)
                    }
                }
            }
        }

        // create new frame
        let mut frame = self.display.draw();
        frame.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
        WindowFrame {
            frame,
            window: self,
        }
    }
}

pub struct WindowFrame<'a> {
    // TODO: do not pub!
    pub frame: glium::Frame,
    pub window: &'a mut Window,
}

impl<'a> WindowFrame<'a> {
    pub fn end(self) -> bool {
        self.frame.finish().unwrap();

        for event in self.window.event_pump.poll_iter() {
            #[allow(clippy::single_match)]
            match event {
                sdl2::event::Event::Quit { .. } => return false,
                _ => (),
            }
        }
        true
    }

    // TODO: Result
    pub fn draw_text(&mut self, text: &str, pos: [f32; 2], scale: [f32; 2], color: [f32; 4]) {
        let text = glium_text::TextDisplay::new(&self.window.text_system, &self.window.font, text);
        //let text_width = text.get_width();

        let (w, h) = self.window.display.get_framebuffer_dimensions();

        #[rustfmt::skip]
        let matrix:[[f32; 4]; 4] = cgmath::Matrix4::new(
            scale[0], 0.0, 0.0, 0.0,
            0.0, scale[1] * (w as f32) / (h as f32), 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            -1.0 + (pos[0] * 2f32 / w as f32), 1.0 - (pos[1] * 2f32 / h as f32), 0.0, 1.0f32,
        ).into();
        glium_text::draw(
            &text,
            &self.window.text_system,
            &mut self.frame,
            matrix,
            (color[0], color[1], color[2], color[3]),
        )
        .unwrap();
    }

    pub fn draw_texture(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        texture: &SrgbTexture2d,
        alpha: bool,
    ) {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
            tex_coords: [f32; 2],
        }
        implement_vertex!(Vertex, position, tex_coords);

        let vertex1 = Vertex {
            position: [x, y + h],
            tex_coords: [0.0, 1.0],
        };
        let vertex2 = Vertex {
            position: [x + w, y + h],
            tex_coords: [1.0, 1.0],
        };
        let vertex3 = Vertex {
            position: [x + w, y],
            tex_coords: [1.0, 0.0],
        };
        let vertex4 = Vertex {
            position: [x, y],
            tex_coords: [0.0, 0.0],
        };
        let shape = vec![vertex1, vertex2, vertex3, vertex4];

        let uniforms = uniform! {
            tex: texture.sampled().magnify_filter(MagnifySamplerFilter::Linear), // Nearest
        };

        let vertex_buffer = glium::VertexBuffer::new(&self.window.display, &shape).unwrap();
        let indices_data: Vec<u16> = vec![0, 1, 2, 0, 2, 3];
        let indices = glium::IndexBuffer::new(
            &self.window.display,
            glium::index::PrimitiveType::TrianglesList,
            &indices_data,
        )
        .unwrap();

        let params = if alpha {
            glium::DrawParameters {
                blend: glium::draw_parameters::Blend::alpha_blending(),
                ..Default::default()
            }
        } else {
            Default::default()
        };

        self.frame
            .draw(
                &vertex_buffer,
                &indices,
                &self.window.program,
                &uniforms,
                &params,
            )
            .unwrap();
    }
}
