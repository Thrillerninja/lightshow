use std::{
    io::{self, Write}, mem::zeroed, thread, time::{Duration, Instant}
};
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
};

use winapi::um::winuser::{GetMonitorInfoW, MONITORINFOEXW};

use crate::FRAME_MAP;

// Struct to hold monitor information
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub monitor: *mut std::ffi::c_void,
    pub pos_x: i32,
    pub pos_y: i32,
    pub width: i32,
    pub height: i32,
}

// Function to parse flags from a string
fn parse_flags(flags: &str) -> Result<(i32, u32), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = flags.split(',').collect();
    if parts.len() != 2 {
        return Err("Invalid flags format".into());
    }
    let id = parts[0].trim().parse::<i32>()?;
    let fps_limit = parts[1].trim().parse::<u32>()?;
    Ok((id, fps_limit))
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
}


// This struct will be used to handle the capture events.
pub struct Capture {
    // Monitor ID
    id: i32,
    // To measure the time the capture has been running
    process_time: Instant,
    // To measure the time between frames
    frame_time: Instant,
    // To count the number of frames captured
    frame_count: u32,
    // To track the last time FPS was logged
    last_fps_log: Instant,
    // Desired FPS limit
    fps_limit: u32,
}

impl GraphicsCaptureApiHandler for Capture {
    // No flags needed for screenshot
    type Flags = String;

    // The type of error that can occur during capture
    type Error = Box<dyn std::error::Error + Send + Sync>;

    // Function that will be called to create the struct. The flags can be passed from settings.
    fn new(flags: Self::Flags) -> Result<Self, Self::Error> {        
        let flags = match parse_flags(&flags) {
            Ok(f) => f,
            Err(_e) => (0, 10),
        };
        
        Ok(            
            Self {
                id: flags.0,
                process_time: Instant::now(),
                frame_time: Instant::now(),
                frame_count: 0,
                last_fps_log: Instant::now(),
                fps_limit: flags.1,
            }
        )
    }

    // Called every time a new frame is available.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        // Increment the frame count
        self.frame_count += 1;

        // Calculate elapsed time since last FPS log
        let elapsed_since_last_log = self.last_fps_log.elapsed();

        // If more than a second has passed, log the FPS
        if elapsed_since_last_log >= Duration::from_secs(1) {
            let fps = self.frame_count as f64 / elapsed_since_last_log.as_secs_f64();
            log::warn!("Monitor {}:: FPS: {:.2}", self.id, fps);

            if self.id == 0 {
                // Print recording
                log::warn!("Monitor {}:: Recording for: {} seconds", 
                    self.id,
                    self.process_time.elapsed().as_secs()
                );
            }

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
            };
            if let Ok(mut map) = FRAME_MAP.lock() {
                map.insert(self.id.clone(), frame_data);
            } else {
                log::error!("Failed to lock FRAME_MAP");
            }
        }

        // ---------- End of processing the frame / cleanup ----------
        io::stdout().flush()?;

        // ---------- FPS Limiting ----------
        // Sleep for a short time to avoid high CPU usage
        if self.fps_limit > 0 {
            // Calc remaining frame time
            let elapsed = self.frame_time.elapsed();
            let frame_duration = Duration::from_secs_f32(1.0 / self.fps_limit as f32);
            if let Some(remaining) = frame_duration.checked_sub(elapsed) {
                if remaining.as_secs_f32() > 0.0 {
                    thread::sleep(remaining);
                    log::warn!("Monitor {}:: Remaining: {:?}", self.id, remaining);
                }
            }
            // Reset frame time after sleeping
            self.frame_time = Instant::now();
        }

        Ok(())
    }

    // Optional handler called when the capture item (usually a window) closes.
    fn on_closed(&mut self) -> Result<(), Self::Error> {
        log::info!("Monitor {}:: Capture Session Closed", self.id);
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