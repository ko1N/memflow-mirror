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
    // a list of all potential capture targets
    //pub targets: CVec<CaptureTarget>,

    // the currenly selected capture target
    //pub current_target: usize,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            gdi: true,
            dxgi: true,
            obs: false,
            //targets: Vec::new().into(),

            //current_target: 0,
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
#[derive(Debug)]
pub struct GlobalBufferGuest {
    pub marker: [u8; 8],
    pub width: u64,
    pub height: u64,
    pub config: CaptureConfig,
    pub frame_counter: u32,
    pub frame_read_counter: u32,
    pub frame_texmode: u8, // TextureMode,
    pub frame_buffer: CVec<u8>,
    pub cursor: Cursor,
    pub screen_index: u32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct GlobalBufferHost {
    pub marker: [u8; 8],
    pub width: u64,
    pub height: u64,
    pub config: CaptureConfig,
    pub frame_counter: u32,
    pub frame_read_counter: u32,
    pub frame_texmode: u8, // TextureMode,
    pub frame_buffer: u64,
    pub frame_buffer_pad: [u8; 32], // padding due to internal layout of CVec<T>
    pub cursor: Cursor,
    pub screen_index: u32,
}
unsafe impl Pod for GlobalBufferHost {}
const _: [(); std::mem::size_of::<GlobalBufferGuest>()] =
    [(); std::mem::size_of::<GlobalBufferHost>()];

impl GlobalBufferGuest {
    pub fn new(resolution: (u64, u64), screen_index: u32) -> Self {
        Self {
            marker: [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
            width: resolution.0,
            height: resolution.1,
            config: CaptureConfig::default(),
            frame_counter: 0,
            frame_read_counter: 0,
            frame_texmode: TextureMode::BGRA as u8, // dxgi default
            frame_buffer: vec![0u8; resolution.0 as usize * resolution.1 as usize * 4].into(),
            cursor: Cursor::default(),
            screen_index,
        }
    }
}

impl GlobalBufferHost {
    pub fn new(resolution: (u64, u64), screen_index: u32) -> Self {
        Self {
            marker: [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
            width: resolution.0,
            height: resolution.1,
            config: CaptureConfig::default(),
            frame_counter: 0,
            frame_read_counter: 0,
            frame_texmode: TextureMode::BGRA as u8, // dxgi default
            frame_buffer: 0,
            frame_buffer_pad: [0u8; 32],
            cursor: Cursor::default(),
            screen_index,
        }
    }
}
