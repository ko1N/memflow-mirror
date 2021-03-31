use std::mem::size_of;
use std::ptr;

use winapi::shared::windef::POINT;
use winapi::um::winuser::{GetCursorInfo, CURSORINFO, CURSOR_SHOWING};

use mirror_dto::Cursor;

pub fn get_state() -> Result<Cursor, &'static str> {
    let mut ci = CURSORINFO {
        cbSize: size_of::<CURSORINFO>() as u32,
        flags: 0,
        hCursor: ptr::null_mut(),
        ptScreenPos: POINT { x: 0, y: 0 },
    };
    let result = unsafe { GetCursorInfo(&mut ci) };
    if result != 0 {
        Ok(Cursor {
            is_visible: if ci.flags == CURSOR_SHOWING { 1 } else { 0 },
            cursor_id: ci.hCursor as u32,
            x: ci.ptScreenPos.x,
            y: ci.ptScreenPos.y,
        })
    } else {
        Err("unable to get cursor info")
    }
}
