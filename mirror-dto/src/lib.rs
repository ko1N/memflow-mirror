#[repr(C)]
pub struct GlobalBuffer {
    _marker: [u8; 8],               // 0x0
    pub resolution: (usize, usize), // 0x8
    pub frame_counter: u32,         // 0x18
    pub frame_buffer: Vec<u8>,      // 0x20
    pub cursor: Cursor,             // 0x28
}

impl GlobalBuffer {
    pub fn new(resolution: (usize, usize)) -> Self {
        Self {
            _marker: [0xD, 0xE, 0xA, 0xD, 0xB, 0xA, 0xB, 0xE],
            resolution,
            frame_counter: 0,
            frame_buffer: vec![0u8; resolution.0 * resolution.1 * 4],
            cursor: Cursor::default(),
        }
    }
}

#[repr(C)]
pub struct Cursor {
    pub is_visible: bool,
    pub cursor_id: u32, // TODO:
    pub x: i32,
    pub y: i32,
}

impl Cursor {
    pub fn new() -> Self {
        Self {
            is_visible: false,
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
