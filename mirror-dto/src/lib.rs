pub use memflow::cglue::prelude::v1::{CVec, ReprCString};
use memflow::prelude::v1::Pod;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum CaptureTargetType {
    Desktop = 0,
    Window = 1,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct CaptureTarget {
    pub ty: u8, // CaptureTargetType,
    pub name: ReprCString,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct CaptureConfig {
    /// allowed capture modes
    pub gdi: bool,
    pub dxgi: bool,
    pub obs: bool,

    /// a list of all potential capture targets
    pub targets: CVec<CaptureTarget>,

    /// the currenly selected capture target
    pub current_target: usize,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            gdi: true,
            dxgi: true,
            obs: true,

            targets: Vec::new().into(),

            current_target: 0,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TextureMode {
    RGBA = 0,
    BGRA = 1,
}

#[repr(C)]
#[derive(Pod, Clone, Copy, Debug)]
pub struct Cursor {
    pub is_visible: i32,
    pub cursor_id: u32, // TODO:
    pub x: i32,
    pub y: i32,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            is_visible: 0,
            cursor_id: 0,
            x: 0,
            y: 0,
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct GlobalBuffer {
    pub marker: [u8; 8],
    pub config: CaptureConfig,
    pub width: usize,
    pub height: usize,
    pub frame_counter: u32,
    pub frame_read_counter: u32,
    pub frame_texmode: u8, // TextureMode,
    pub frame_buffer: CVec<u8>,
    pub cursor: Cursor,
    pub screen_index: usize,
}
unsafe impl Pod for GlobalBuffer {}

impl GlobalBuffer {
    pub fn new(resolution: (usize, usize), screen_index: usize) -> Self {
        Self {
            marker: [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
            config: CaptureConfig::default(),
            width: resolution.0,
            height: resolution.1,
            frame_counter: 0,
            frame_read_counter: 0,
            frame_texmode: TextureMode::BGRA as u8, // dxgi default
            frame_buffer: vec![0u8; resolution.0 * resolution.1 * 4].into(),
            cursor: Cursor::default(),
            screen_index,
        }
    }
}
