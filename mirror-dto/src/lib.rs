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
pub struct Frame {
    pub texture_mode: u8, // TextureMode,
    pub buffer: CVec<u8>,
}
unsafe impl Pod for Frame {}

impl Frame {
    pub fn new(resolution: (usize, usize)) -> Self {
        Self {
            texture_mode: TextureMode::BGRA as u8, // dxgi default
            buffer: vec![0u8; resolution.0 * resolution.1 * 4].into(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct GlobalBuffer {
    pub marker: [u8; 8],
    pub config: CaptureConfig,
    pub width: usize,
    pub height: usize,

    /// A vec containg the frames that are currently being read and written to
    pub frame0: Frame,
    pub frame1: Frame,
    /// The frame thats currently being written by the mirror-guest - flips between indizes of `frames`
    pub write_frame: usize,
    /// The frame thats currently being read by the mirror - flips between indizes of `frames`
    pub read_frame: usize,

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

            frame0: Frame::new(resolution),
            frame1: Frame::new(resolution),
            write_frame: 0,
            read_frame: 0,

            cursor: Cursor::default(),
            screen_index,
        }
    }
}
