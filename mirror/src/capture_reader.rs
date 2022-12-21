use ::log::{info, warn};
use ::std::{
    convert::TryInto,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    thread::JoinHandle,
};
use frame_counter::FrameCounter;
use mirror_dto::{GlobalBuffer, TextureMode};
use parking_lot::RwLock;

use ::memflow::prelude::v1::*;

pub struct CaptureReader {
    thread_handle: Option<JoinHandle<()>>,
    thread_alive: Arc<AtomicBool>,

    // synced with main thread
    capture_data: Arc<RwLock<CaptureData>>,
}

impl CaptureReader {
    pub fn new(os: OsInstanceArcBox<'static>) -> Self {
        let capture_data = Arc::new(RwLock::new(CaptureData::default()));
        let mut inner = CaptureReaderInner::new(os, capture_data.clone());

        let mut reader = Self {
            thread_handle: None,
            thread_alive: Arc::new(AtomicBool::new(true)),

            capture_data,
        };

        let alive = reader.thread_alive.clone();
        reader.thread_handle = Some(thread::spawn(move || {
            info!("processing thread created",);

            // run node processing
            while alive.load(Ordering::SeqCst) {
                // TODO: run process function
                inner.process();
            }

            info!("processing thread destroyed",);
        }));

        reader
    }

    pub fn image_data(&self) -> egui::ImageData {
        let (frame_width, frame_height, mut frame_buffer) = {
            let capture_data = self.capture_data.read();
            (
                capture_data.global_buffer.width,
                capture_data.global_buffer.height,
                capture_data.frame_buffer.clone(),
            )
        };

        let size = [frame_width as usize, frame_height as usize];
        let mut data = std::mem::ManuallyDrop::new(frame_buffer);
        let pixels: Vec<egui::Color32> = unsafe {
            Vec::from_raw_parts(
                data.as_mut_ptr() as *mut _,
                data.len() / std::mem::size_of::<egui::Color32>(),
                data.len() / std::mem::size_of::<egui::Color32>(),
            )
        };

        egui::ImageData::Color(egui::ColorImage { size, pixels })
    }
}

impl Drop for CaptureReader {
    fn drop(&mut self) {
        self.thread_alive.store(false, Ordering::SeqCst);
        if self
            .thread_handle
            .take()
            .expect("Called stop on non-running thread")
            .join()
            .is_err()
        {
            warn!("Could not join thread for worker node");
        }
    }
}

struct CaptureReaderInner {
    os: OsInstanceArcBox<'static>,
    process: Option<CaptureProcess>,
    capture_data: Arc<RwLock<CaptureData>>,
    update_counter: FrameCounter,
}

impl CaptureReaderInner {
    pub fn new(os: OsInstanceArcBox<'static>, capture_data: Arc<RwLock<CaptureData>>) -> Self {
        Self {
            os,
            process: None,
            capture_data,
            update_counter: FrameCounter::new(0f64),
        }
    }

    pub fn process(&mut self) -> () {
        if let Some(process) = &mut self.process {
            if process.update().is_ok() {
                self.update_counter.tick();
            }
        } else {
            // try to open the process
            if let Ok(capture_process) = CaptureProcess::new(
                self.os.clone(),
                "mirror-guest.exe",
                self.capture_data.clone(),
            ) {
                self.process = Some(capture_process);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}

struct CaptureData {
    global_buffer: GlobalBuffer,
    frame_buffer: Vec<u8>,
}

impl Default for CaptureData {
    fn default() -> Self {
        // pre-allocate buffer with a common resolution
        Self {
            global_buffer: GlobalBuffer::new((1920, 1080), 0),
            frame_buffer: vec![0u8; 1920 * 1080 * 4],
        }
    }
}

struct CaptureProcess {
    process: IntoProcessInstanceArcBox<'static>,
    module_info: ModuleInfo,
    marker_addr: Address,

    capture_data: Arc<RwLock<CaptureData>>,

    // internal
    frame_width: u32,
    frame_height: u32,
    frame_counter: u32,
}

impl CaptureProcess {
    pub fn new(
        os: OsInstanceArcBox<'static>,
        process_name: &str,
        capture_data: Arc<RwLock<CaptureData>>,
    ) -> Result<Self> {
        let mut process = os
            .into_process_by_name(process_name)
            .map_err(|e| e.log_error("unable to find memflow mirror guest process"))?;
        info!("found process: {:?}", process.info());

        let module_info = process
            .module_by_name(process_name)
            .map_err(|e| e.log_error("unable to find memflow mirror guest module in process"))?;
        info!("found module: {:?}", module_info);

        // read entire module for sigscanning
        let module_buf = process
            .read_raw(module_info.base, module_info.size.try_into().unwrap())
            .data_part()
            .map_err(|e| e.log_error("unable to read module"))?;

        let marker_offs = Self::find_marker(&module_buf)
            .map_err(|e| e.log_error("unable to find marker in binary"))?;
        info!("marker found at {:x} + {:x}", module_info.base, marker_offs);
        let marker_addr = module_info.base + marker_offs;

        // read global_buffer object from guest
        let (frame_width, frame_height) = {
            let mut capture_data = capture_data.write();
            process.read_into(marker_addr, &mut capture_data.global_buffer)?;

            info!(
                "found resolution: {}x{}",
                capture_data.global_buffer.width, capture_data.global_buffer.height
            );
            info!(
                "found frame_buffer addr: {:x}",
                capture_data.global_buffer.frame_buffer.as_ptr() as umem
            );

            (
                capture_data.global_buffer.width as u32,
                capture_data.global_buffer.height as u32,
            )
        };

        Ok(Self {
            process,
            module_info,
            marker_addr,

            capture_data,

            frame_width,
            frame_height,
            frame_counter: 0,
        })
    }

    fn find_marker(module_buf: &[u8]) -> Result<usize> {
        use ::regex::bytes::*;

        // 0D 0E 0A 0D 0B 0A 0B 0E ? ? ? ? 0 0 0 0
        // since the global buffer contains 2 resolution values (width and height) right after the marker
        // and the resolution is definatly smaller than u32::MAX we can narrow down the search
        // by adding those trailing 0's to the scan
        let re = Regex::new("(?-u)\\x0D\\x0E\\x0A\\x0D\\x0B\\x0A\\x0B\\x0E(?s:.)(?s:.)(?s:.)(?s:.)\\x00\\x00\\x00\\x00(?s:.)(?s:.)(?s:.)(?s:.)\\x00\\x00\\x00\\x00")
            .expect("malformed marker signature");
        let buf_offs = re
            .find_iter(module_buf)
            .next()
            .ok_or_else(|| Error(ErrorOrigin::VirtualMemory, ErrorKind::NotFound))?
            .start();

        Ok(buf_offs as usize)
    }

    pub fn update(&mut self) -> Result<()> {
        // check if a frame buffer is necessary
        let (frame_width, frame_height, frame_counter) = {
            let mut capture_data = self.capture_data.write();
            self.process
                .read_into(self.marker_addr, &mut capture_data.global_buffer)?;
            (
                capture_data.global_buffer.width as u32,
                capture_data.global_buffer.height as u32,
                capture_data.global_buffer.frame_counter,
            )
        };

        if frame_counter == self.frame_counter {
            // no new update yet
            return Err(Error(ErrorOrigin::VirtualMemory, ErrorKind::AlreadyExists));
        }

        // check if resolution has been changed
        if self.frame_width != frame_width as u32 || self.frame_height != frame_height as u32 {
            // limit to 16k resolution
            if frame_width <= 15360 && frame_height <= 8640 {
                info!("changing resolution: to {}x{}", frame_width, frame_height);
                {
                    let mut capture_data = self.capture_data.write();
                    capture_data.frame_buffer =
                        vec![0u8; (frame_width * frame_height * 4) as usize];
                }
                /*
                let new_image = glium::texture::RawImage2d::from_raw_rgba(
                    self.frame_buffer.clone(),
                    (self.global_buffer.width as u32, self.global_buffer.height as u32),
                );
                self.texture = glium::texture::SrgbTexture2d::new(&wnd.display, new_image).unwrap();
                */
            }
        }

        {
            // update frame_buffer on host
            let mut capture_data = self.capture_data.write();
            self.process
                .read_into(
                    (capture_data.global_buffer.frame_buffer.as_mut_ptr() as umem).into(),
                    &mut capture_data.frame_buffer[..],
                )
                .ok();

            // update configuration on guest
            capture_data.global_buffer.config.obs = false; // TODO: enable_obs; // TODO: more configuration
            capture_data.global_buffer.frame_read_counter =
                capture_data.global_buffer.frame_counter;
            self.process
                .write(self.marker_addr, &capture_data.global_buffer)
                .ok();
        }

        // update image
        /*
        let new_image = glium::texture::RawImage2d::from_raw_rgba(
            self.frame_buffer.clone(),
            (self.global_buffer.width as u32, self.global_buffer.height as u32),
        );
        */

        // update texture
        /*
        self.texture.write(
            glium::Rect {
                left: 0,
                bottom: 0,
                width: global_buffer.width as u32,
                height: global_buffer.height as u32,
            },
            new_image,
        );
        */

        self.frame_counter = frame_counter;

        Ok(()) // TODO:
    }
}
