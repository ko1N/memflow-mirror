mod dxgi;
use dxgi::DXGIManager;

use std::slice;

use mirror_dto::{TextureMode, CVec};

#[derive(Clone, PartialEq)]
pub enum CaptureMode {
    //BitBlt,
    DXGI,
    OBS(String),
}

pub struct Capture {
    mode: CaptureMode,

    resolution: (usize, usize),

    // TODO: bitblt
    dxgi: Option<DXGIManager>,
    obs: Option<obs_client::Capture>,
}

impl Capture {
    pub fn new() -> Result<Self, String> {
        let dxgi = DXGIManager::new(1000)?;
        let resolution = dxgi.geometry();
        Ok(Self {
            mode: CaptureMode::DXGI,

            resolution,

            dxgi: Some(dxgi),
            obs: None,
        })
    }

    pub fn resolution(&self) -> (usize, usize) {
        // TODO: update resolution
        self.resolution
    }

    pub fn mode(&self) -> CaptureMode {
        self.mode.clone()
    }

    pub fn set_mode(&mut self, mode: CaptureMode) -> Result<(), String> {
        match &mode {
            CaptureMode::DXGI => {
                self.dxgi = Some(DXGIManager::new(1000)?);
                self.obs = None;
                self.mode = mode;
                Ok(())
            }
            CaptureMode::OBS(window_name) => {
                let mut obs = obs_client::Capture::new(window_name);
                if obs.try_launch().is_err() {
                    return Err("Failed to enable obs capture".to_string());
                }
                self.dxgi = None;
                self.obs = Some(obs);
                self.mode = mode;
                Ok(())
            }
        }
    }

    // TODO: let this directly write into the out buffer which safes additional allocations.
    pub fn capture_frame(&mut self) -> Result<Frame<'_>, String> {
        match self.mode {
            CaptureMode::DXGI => Ok(Frame::DXGI(
                self.dxgi
                    .as_mut()
                    .unwrap()
                    .capture_frame::<u8>()
                    .map_err(|_| "unable to capture frame".to_string())?,
            )),
            CaptureMode::OBS(_) => Ok(Frame::OBS(
                self.obs
                    .as_mut()
                    .unwrap()
                    .capture_frame::<u8>()
                    .map_err(|_| "unable to capture frame".to_string())?,
            )),
        }
    }
}

pub enum Frame<'a> {
    DXGI((&'a [u8], (usize, usize))),
    OBS((&'a mut [u8], (usize, usize))),
}

impl<'a> Frame<'a> {
    pub fn resolution(&self) -> (usize, usize) {
        match self {
            Frame::DXGI((_, resolution)) => *resolution,
            Frame::OBS((_, resolution)) => *resolution,
        }
    }

    pub fn buffer_len(&self) -> usize {
        match self {
            Frame::DXGI((buffer, _)) => buffer.len(),
            Frame::OBS((buffer, _)) => buffer.len(),
        }
    }

    pub fn texture_mode(&self) -> TextureMode {
        match self {
            Frame::DXGI(_) => TextureMode::BGRA,
            Frame::OBS(_) => TextureMode::RGBA,
        }
    }

    pub unsafe fn copy_frame(&self, to: &mut CVec<u8>) {
        match self {
            Frame::DXGI((buffer, _)) => {
                to.copy_from_slice(slice::from_raw_parts(
                    buffer.as_ptr() as *const u8,
                    buffer.len(),
                ));
            }
            Frame::OBS((buffer, _)) => {
                to.copy_from_slice(slice::from_raw_parts(
                    buffer.as_ptr() as *const u8,
                    buffer.len(),
                ));
            }
        }
    }
}
