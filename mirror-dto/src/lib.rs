#[repr(C)]
pub struct GlobalBuffer {
    _marker: [u8; 8],           // 0x0
    resolution: (usize, usize), // 0x8
    frame_counter: u32,         // 0x18
    frame_buffer: Vec<u8>,      // 0x20
    cursor: Cursor,             // 0x28
}

#[repr(C)]
pub struct Cursor {
    is_visible: bool,
    cursor_id: u32, // TODO:
    x: i32,
    y: i32,
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
