use memflow::prelude::v1::Pod;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum TextureMode {
    RGBA = 0,
    BGRA = 1,
}

#[repr(C)]
pub struct GlobalBuffer {
    pub marker: [u8; 8],            // 0x0
    pub width: usize,               // 0x8
    pub height: usize,              // 0x10
    pub frame_counter: u32,         // 0x18
    pub frame_read_counter: u32,    // 0x20
    pub frame_texmode: TextureMode, //
    pub frame_buffer: Vec<u8>,      // 0x28
    pub cursor: Cursor,             //
    pub screen_index: usize,
}

#[repr(C)]
pub struct GlobalBufferRaw {
    pub marker: [u8; 8],            // 0x0
    pub width: usize,               // 0x8
    pub height: usize,              // 0x10
    pub frame_counter: u32,         // 0x18
    pub frame_read_counter: u32,    // 0x20
    pub frame_texmode: TextureMode, //
    pub frame_buffer: u64,          // 0x28
    pad0: [u8; 0x10],               //
    pub cursor: Cursor,             //
    pub screen_index: usize,
}
unsafe impl Pod for GlobalBufferRaw {}

impl GlobalBuffer {
    pub fn new(resolution: (usize, usize), screen_index: usize) -> Self {
        Self {
            marker: [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
            width: resolution.0,
            height: resolution.1,
            frame_counter: 0,
            frame_read_counter: 0,
            frame_texmode: TextureMode::BGRA, // dxgi default
            frame_buffer: vec![0u8; resolution.0 * resolution.1 * 4],
            cursor: Cursor::default(),
            screen_index,
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
