use memflow::prelude::v1::{Pod};

#[repr(C)]
pub struct GlobalBuffer {
    _marker: [u8; 8],          // 0x0
    pub width: usize,          // 0x8
    pub height: usize,         // 0x10
    pub frame_counter: u32,    // 0x18
    pub frame_buffer: Vec<u8>, // 0x20
    pub cursor: Cursor,        // 0x38
}

#[repr(C)]
pub struct GlobalBufferRaw {
    _marker: [u8; 8],       // 0x0
    pub width: usize,       // 0x8
    pub height: usize,      // 0x10
    pub frame_counter: u32, // 0x18
    pub frame_buffer: u64,  // 0x20
    pad0: [u8; 0x10],
    pub cursor: Cursor, // 0x38
}
unsafe impl Pod for GlobalBufferRaw {}

impl GlobalBuffer {
    pub fn new(resolution: (usize, usize)) -> Self {
        Self {
            _marker: [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
            width: resolution.0,
            height: resolution.1,
            frame_counter: 0,
            frame_buffer: vec![0u8; resolution.0 * resolution.1 * 4],
            cursor: Cursor::default(),
        }
    }
}

#[repr(C)]
#[derive(Pod)]
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
