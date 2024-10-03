use std::{
    io::{self, Write}, mem::zeroed, sync::{Arc, Mutex}, thread, time::{Duration, Instant}
};

use concurrent_queue::ConcurrentQueue;
use image::RgbaImage;
use windows_capture::frame::ImageFormat;
use log::{error, info};
use once_cell::sync::Lazy;
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    encoder::{AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
};

use winapi::um::winuser::{GetMonitorInfoW, MONITORINFOEXW};

use crate::{arduino, config::{Config, LED}, screen_capture::process_edge_color, FRAME_QUEUE, FRAME_MAP};

// Struct to hold monitor information
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub monitor: *mut std::ffi::c_void,
    pub pos_x: i32,
    pub pos_y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct SlimMonitorInfo {
    pub pos_x: i32,
    pub pos_y: i32,
    pub width: i32,
    pub height: i32,
}

impl MonitorInfo {
    pub fn export(&self) -> SlimMonitorInfo {
        SlimMonitorInfo {
            pos_x: self.pos_x,
            pos_y: self.pos_y,
            width: self.width,
            height: self.height,
        }
    }
}

// Struct to hold captured frame data
#[derive(Debug, Clone)]
pub struct FrameData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}


// This struct will be used to handle the capture events.
pub struct Capture {
    // The video encoder that will be used to encode the frames.
    encoder: Option<VideoEncoder>,
    // To measure the time the capture has been running
    start: Instant,
    // To count the number of frames captured
    frame_count: u32,
    // To track the last time FPS was logged
    last_fps_log: Instant,
    // Flags to identify the monitor
    flags: i32,
}

impl GraphicsCaptureApiHandler for Capture {
    // No flags needed for screenshot
    type Flags = String;

    // The type of error that can occur during capture
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function that will be called to create the struct. The flags can be passed from settings.
    fn new(flags: Self::Flags) -> Result<Self, Self::Error> {        
        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(1920, 1080),
            AudioSettingsBuilder::default().disabled(true),
            ContainerSettingsBuilder::default(),
            format!("video{}.mp4", flags),
        )?;

        Ok(Self {
            encoder: Some(encoder),
            start: Instant::now(),
            frame_count: 0,
            last_fps_log: Instant::now(),
            flags: flags.parse().unwrap(),
        })
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // Increment the frame count
        self.frame_count += 1;

        // Calculate elapsed time since last FPS log
        let elapsed_since_last_log = self.last_fps_log.elapsed();

        // If more than a second has passed, log the FPS
        if elapsed_since_last_log >= Duration::from_secs(1) {
            let fps = self.frame_count as f64 / elapsed_since_last_log.as_secs_f64();
            log::info!("Monitor {}:: FPS: {:.2}", self.flags, fps);
            // Print recording duration every frame (optional)
            log::info!("Monitor {}:: Recording for: {} seconds", 
                self.flags,
                self.start.elapsed().as_secs()
            );

            io::stdout().flush()?;

            // Reset frame count and update last FPS log time
            self.frame_count = 0;
            self.last_fps_log = Instant::now();
        }

        

        // ---------- Processing the frame ----------
        // ---------- Enqueue the frame ----------
        if let Ok(mut buffer) = frame.buffer() {
            let frame_bytes = match buffer.as_raw_nopadding_buffer() {
                Ok(raw_buffer) => raw_buffer.to_vec(),
                Err(e) => {
                    log::error!("Failed to get raw buffer: {}", e);
                    return Err(Box::new(e));
                }
            };
            let frame_data = FrameData {
                data: frame_bytes,
                width: frame.width(),
                height: frame.height(),
            };
            //FRAME_QUEUE.push(frame_data).ok(); // Enqueue the frame
            if let Ok(mut map) = FRAME_MAP.lock() {
                map.insert(self.flags.clone(), frame_data);
            } else {
                log::error!("Failed to lock FRAME_MAP");
            }
        }

        frame.save_as_image(&format!("frame_{}.png", self.flags), ImageFormat::Png)?;

        // ---------- End of processing the frame ----------

        io::stdout().flush()?;

        // Sleep for a short time to avoid high CPU usage
        thread::sleep(Duration::from_millis(10));

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) closes.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        log::info!("Monitor {}:: Capture Session Closed", self.flags);
        Ok(())
    }
}

// Function to retrieve monitor information
pub fn get_monitor_info() -> Result<Vec<MonitorInfo>, Box<dyn std::error::Error>> {
    let monitors = Monitor::enumerate()?;
    let mut monitor_info_list = Vec::new();

    for monitor in monitors.iter() {
        let mut mi: MONITORINFOEXW = unsafe { zeroed() };
        unsafe {
            mi.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if GetMonitorInfoW(monitor.as_raw_hmonitor() as *mut _, &mut mi as *mut _ as *mut _) != 0 {
                let x = mi.rcMonitor.left;
                let y = mi.rcMonitor.top;
                let width: i32 = mi.rcMonitor.right - mi.rcMonitor.left;
                let height = mi.rcMonitor.bottom - mi.rcMonitor.top;

                monitor_info_list.push(MonitorInfo {
                    monitor: monitor.as_raw_hmonitor(),
                    pos_x: x,
                    pos_y: y,
                    width,
                    height,
                });
            }
        }
    }

    Ok(monitor_info_list)
}

// Function to capture a single screenshot
//pub fn capture_screen(queue: &ConcurrentQueue<FrameData>) -> Result<(), Box<dyn std::error::Error>> {
//    // Enumerate monitors and select the first one
//    let monitors = Monitor::enumerate()?;
//    if monitors.is_empty() {
//        return Err("No monitors found".into());
//    }
//    let monitor = &monitors[0]; // Capture the first monitor for simplicity
//
//    // Create capture settings
//    // Gets The Foreground Window, Checkout The Docs For Other Capture Items
//    let primary_monitor = Monitor::primary().expect("There is no primary monitor");
//
//    let settings = Settings::new(
//        // Item To Capture
//        primary_monitor,
//        // Capture Cursor Settings
//        CursorCaptureSettings::Default,
//        // Draw Borders Settings
//        DrawBorderSettings::WithoutBorder,
//        // The desired color format for the captured frame.
//        ColorFormat::Rgba8,
//        // Additional flags for the capture settings that will be passed to user defined `new` function.
//        queue,
//    );
//
//    // Starts the capture and takes control of the current thread.
//    // The errors from handler trait will end up here
//    Capture::start(settings).expect("Screen Capture Failed");
//    println!("Capture started");
//
//    Ok(())
//}