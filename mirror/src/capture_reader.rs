use ::log::{info, warn};
use ::std::{
    convert::TryInto,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    thread::JoinHandle,
};
use frame_counter::FrameCounter;
use mirror_dto::{CaptureConfig, GlobalBufferHost};
use parking_lot::RwLock;

use ::memflow::prelude::v1::*;

const DEFAULT_FRAME_WIDTH: u64 = 1920;
const DEFAULT_FRAME_HEIGHT: u64 = 1080;

pub trait Capture {
    // Is this a multithreaded reader?
    fn multithreading(&self) -> bool;

    // Consumes self and returns the underlying os object
    fn os(&self) -> OsInstanceArcBox<'static>;

    fn obs_capture(&self) -> bool;
    fn set_obs_capture(&mut self, obs: bool);

    // Returns a new egui::ImageData from the captured data
    fn image_data(&mut self) -> egui::ImageData;
}

pub struct SequentialCapture {
    os: OsInstanceArcBox<'static>,

    process: Option<CaptureProcess>,
    capture_config: CaptureConfig,
    capture_data: CaptureData,
    update_counter: FrameCounter,
}

impl SequentialCapture {
    pub fn new(os: OsInstanceArcBox<'static>) -> Self {
        Self {
            os,

            process: None,
            capture_config: CaptureConfig::default(),
            capture_data: CaptureData::default(),
            update_counter: FrameCounter::new(0f64),
        }
    }

    fn process(&mut self) {
        if let Some(process) = &mut self.process {
            if process.is_alive() {
                if process
                    .update_into(&self.capture_config, &mut self.capture_data)
                    .is_ok()
                {
                    self.update_counter.tick();
                }
            } else {
                self.process = None;
            }
        } else {
            // try to open the process
            if let Ok(capture_process) = CaptureProcess::new(self.os.clone(), "mirror-guest.exe") {
                self.process = Some(capture_process);
            }
        }
    }
}

impl Capture for SequentialCapture {
    fn multithreading(&self) -> bool {
        false
    }

    // Consumes self and returns the underlying os object
    fn os(&self) -> OsInstanceArcBox<'static> {
        self.os.clone()
    }

    fn obs_capture(&self) -> bool {
        self.capture_config.obs
    }
    fn set_obs_capture(&mut self, obs: bool) {
        self.capture_config.obs = obs;
    }

    fn image_data(&mut self) -> egui::ImageData {
        self.process();

        let (frame_width, frame_height, frame_buffer) = {
            (
                self.capture_data.global_buffer.width,
                self.capture_data.global_buffer.height,
                self.capture_data.frame_buffer.clone(),
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

pub struct ThreadedCapture {
    os: OsInstanceArcBox<'static>,

    thread_handle: Option<JoinHandle<()>>,
    thread_alive: Arc<AtomicBool>,

    // synced with main thread
    capture_config: Arc<RwLock<CaptureConfig>>,
    capture_data: Arc<RwLock<CaptureData>>,
}

impl ThreadedCapture {
    pub fn new(os: OsInstanceArcBox<'static>) -> Self {
        let capture_config = Arc::new(RwLock::new(CaptureConfig::default()));
        let capture_data = Arc::new(RwLock::new(CaptureData::default()));
        let mut inner =
            ThreadedCaptureInner::new(os.clone(), capture_config.clone(), capture_data.clone());

        let mut reader = Self {
            os,

            thread_handle: None,
            thread_alive: Arc::new(AtomicBool::new(true)),

            capture_config,
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
}

impl Capture for ThreadedCapture {
    fn multithreading(&self) -> bool {
        true
    }

    fn os(&self) -> OsInstanceArcBox<'static> {
        self.os.clone()
    }

    fn obs_capture(&self) -> bool {
        self.capture_config.read().obs
    }
    fn set_obs_capture(&mut self, obs: bool) {
        self.capture_config.write().obs = obs;
    }

    fn image_data(&mut self) -> egui::ImageData {
        let (frame_width, frame_height, frame_buffer) = {
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

impl Drop for ThreadedCapture {
    fn drop(&mut self) {
        if self.thread_handle.is_some() {
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
}

struct ThreadedCaptureInner {
    os: OsInstanceArcBox<'static>,
    process: Option<CaptureProcess>,
    capture_config: Arc<RwLock<CaptureConfig>>,
    capture_data: Arc<RwLock<CaptureData>>,
    update_counter: FrameCounter,
}

impl ThreadedCaptureInner {
    pub fn new(
        os: OsInstanceArcBox<'static>,
        capture_config: Arc<RwLock<CaptureConfig>>,
        capture_data: Arc<RwLock<CaptureData>>,
    ) -> Self {
        Self {
            os,
            process: None,
            capture_config,
            capture_data,
            update_counter: FrameCounter::new(0f64),
        }
    }

    pub fn process(&mut self) -> () {
        if let Some(process) = &mut self.process {
            if process.is_alive() {
                if process
                    .update_into(&self.capture_config.read(), &mut self.capture_data.write())
                    .is_ok()
                {
                    self.update_counter.tick();
                }
            } else {
                self.process = None;
            }
        } else {
            // try to open the process
            if let Ok(capture_process) = CaptureProcess::new(self.os.clone(), "mirror-guest.exe") {
                self.process = Some(capture_process);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}

struct CaptureData {
    global_buffer: GlobalBufferHost,
    frame_buffer: Vec<u8>,
}

impl Default for CaptureData {
    fn default() -> Self {
        // pre-allocate buffer with a common resolution
        Self {
            global_buffer: GlobalBufferHost::new((DEFAULT_FRAME_WIDTH, DEFAULT_FRAME_HEIGHT), 0),
            frame_buffer: vec![
                0u8;
                DEFAULT_FRAME_WIDTH as usize * DEFAULT_FRAME_HEIGHT as usize * 4
            ],
        }
    }
}

struct CaptureProcess {
    process: IntoProcessInstanceArcBox<'static>,
    marker_addr: Address,

    // internal
    frame_width: u32,
    frame_height: u32,
    frame_counter: u32,
}

impl CaptureProcess {
    pub fn new(mut os: OsInstanceArcBox<'static>, process_name: &str) -> Result<Self> {
        let mut processes = vec![];
        let callback = &mut |data: ProcessInfo| {
            if data.name.as_ref() == process_name {
                processes.push(data);
            }
            true
        };
        os.process_info_list_callback(callback.into())?;

        for process_info in processes.iter() {
            let mut process = match os.clone().into_process_by_info(process_info.clone()) {
                Ok(process) => process,
                Err(_) => continue,
            };
            info!("found process: {:?}", process_info);

            let module_info = match process.module_by_name(process_name) {
                Ok(module_info) => module_info,
                Err(err) => {
                    err.log_error("unable to find memflow mirror guest module in process");
                    continue;
                }
            };
            info!("found module: {:?}", module_info);

            // read entire module for sigscanning
            let module_buf = match process
                .read_raw(module_info.base, module_info.size.try_into().unwrap())
                .data_part()
            {
                Ok(module_buf) => module_buf,
                Err(err) => {
                    err.log_error("unable to read module");
                    continue;
                }
            };

            let marker_offs = match Self::find_marker(&module_buf) {
                Ok(marker_offs) => marker_offs,
                Err(err) => {
                    err.log_error("unable to find marker in binary");
                    continue;
                }
            };
            info!("marker found at {:x} + {:x}", module_info.base, marker_offs);
            let marker_addr = module_info.base + marker_offs;

            return Ok(Self {
                process,
                marker_addr,

                frame_width: 0,
                frame_height: 0,
                frame_counter: 0,
            });
        }

        Err(Error(ErrorOrigin::OsLayer, ErrorKind::NotFound))
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

    pub fn is_alive(&mut self) -> bool {
        self.process.state() == ProcessState::Alive
    }

    pub fn update_into(
        &mut self,
        capture_config: &CaptureConfig,
        capture_data: &mut CaptureData,
    ) -> Result<()> {
        // check if a new buffer is necessary
        self.process
            .read_into(self.marker_addr, &mut capture_data.global_buffer)?;
        let frame_width = capture_data.global_buffer.width as u32;
        let frame_height = capture_data.global_buffer.height as u32;
        let frame_counter = capture_data.global_buffer.frame_counter;

        if frame_counter == self.frame_counter {
            // no new update yet
            return Err(Error(ErrorOrigin::VirtualMemory, ErrorKind::AlreadyExists));
        }

        // check if resolution has been changed
        if self.frame_width != frame_width as u32 || self.frame_height != frame_height as u32 {
            // limit to 16k resolution
            if frame_width <= 15360 && frame_height <= 8640 {
                info!("changing resolution: to {}x{}", frame_width, frame_height);
                capture_data.frame_buffer = vec![0u8; (frame_width * frame_height * 4) as usize];
                self.frame_width = frame_width;
                self.frame_height = frame_height;
            }
        }

        // update frame_buffer on host
        self.process
            .read_into(
                (capture_data.global_buffer.frame_buffer as umem).into(),
                &mut capture_data.frame_buffer[..],
            )
            .ok();

        // update configuration on guest
        capture_data.global_buffer.config = capture_config.clone();
        capture_data.global_buffer.frame_read_counter = capture_data.global_buffer.frame_counter;
        self.process
            .write(self.marker_addr, &capture_data.global_buffer)
            .ok();

        self.frame_counter = frame_counter;

        Ok(())
    }
}
