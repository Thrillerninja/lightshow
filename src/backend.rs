use crate::arduino;
use crate::config;
use crate::hardware_interaction::{get_monitor_info, SlimMonitorInfo};
use crate::hardware_interaction::{Capture, FrameData};
use crate::logger;
use crate::screen_capture::{calculate_avg_colors, combine_screens};
use crate::SharedState;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc::Receiver},
    thread,
    time::{Duration, Instant},
    sync::atomic::Ordering,
};
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    monitor::Monitor,
    settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
};

pub static FRAME_MAP: Lazy<Arc<Mutex<HashMap<i32, FrameData>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static CONFIG: Lazy<config::Config> =
    Lazy::new(|| config::read_config("0current_config.txt").expect("Failed to read config file"));

pub fn main_program_start(shared_state: Arc<Mutex<SharedState>>) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging (optional)
    // logger::init_logger()?;

    let target_fps = 24;

    // Retrieve monitor information (if needed)
    let monitors = get_monitor_info()?;
    println!("Monitors: {:?}", monitors);

    // Start the processing thread
    let processing_start = Instant::now();
    let processing_handle = process_frames_setup_map(
        monitors.clone().into_iter().map(|m| m.export()).collect(),
        target_fps,
        Arc::clone(&shared_state),
    );
    let processing_duration = processing_start.elapsed();
    println!("Processing thread setup took: {:?}", processing_duration);

    // Start capture for each monitor
    let mut capture_handles = Vec::new();
    for (i, monitor_info) in monitors.into_iter().enumerate() {
        let monitor_handle = Monitor::from_raw_hmonitor(monitor_info.monitor);
        let capture_start = Instant::now();
        let capture_handle = thread::spawn(move || {
            let settings = Settings::new(
                monitor_handle,
                CursorCaptureSettings::Default,
                DrawBorderSettings::WithoutBorder,
                ColorFormat::Rgba8,
                format!("{},{}", i, target_fps),
            );

            // Start the capture and handle potential failures
            if let Err(e) = Capture::start(settings) {
                log::error!("Screen Capture Failed at monitor {}: {:?}", i, e);
                return;
            }
            println!("Capture started for monitor {:?}", i);
        });

        capture_handles.push(capture_handle);
        let capture_duration = capture_start.elapsed();
        println!(
            "Capture thread setup for monitor {} took: {:?}",
            i, capture_duration
        );
    }
    Ok(())
}

fn process_frames_setup_map(
    monitors: Vec<SlimMonitorInfo>,
    target_fps: u32,
    shared_state: Arc<Mutex<SharedState>>
) -> Vec<thread::JoinHandle<()>> {
    let num_threads = 1; // Number of threads for processing
    let mut handles = Vec::with_capacity(num_threads);

    let combined_monitor_width: i32 = monitors.iter().map(|m| m.width).sum();
    let combined_monitor_height: i32 = monitors.iter().map(|m| m.height).max().unwrap();

    for thread_num in 0..num_threads {
        let value: Vec<SlimMonitorInfo> = monitors.clone();
        let shared_state = Arc::clone(&shared_state);
        let handle = thread::spawn(move || {
            let min_x = value.iter().map(|mi| mi.pos_x).min().unwrap_or(0);
            let min_y = value.iter().map(|mi| mi.pos_y).min().unwrap_or(0);
            let max_x = value
                .iter()
                .map(|mi| mi.pos_x + mi.width)
                .max()
                .unwrap_or(0);
            let max_y = value
                .iter()
                .map(|mi| mi.pos_y + mi.height)
                .max()
                .unwrap_or(0);
            log::info!(
                "Combined Screen dimensions:: min_x: {}, min_y: {}, max_x: {}, max_y: {}",
                min_x,
                min_y,
                max_x,
                max_y
            );

            loop {

                let loop_start = Instant::now(); // Start timing the loop

                let combined_img = combine_screens(
                    &value,
                    combined_monitor_width as u32,
                    combined_monitor_height as u32,
                    thread_num as u32,
                    min_x,
                    min_y,
                )
                .unwrap();

                let avg_colors_start = Instant::now();
                let mut avg_colors = calculate_avg_colors(
                    &combined_img,
                    min_x,
                    min_y,
                    max_x,
                    max_y,
                    &CONFIG.leds_array,
                )
                .unwrap();
                let avg_colors_duration = avg_colors_start.elapsed();
                log::info!(
                    "Thread {}:: Average color calculation took: {:?}",
                    thread_num,
                    avg_colors_duration
                );

                // Sort the average colors by LED index
                let avg_colors_start = Instant::now();
                avg_colors.sort_by(|a, b| a.led_index.cmp(&b.led_index));
                let avg_colors_duration = avg_colors_start.elapsed();
                log::info!(
                    "Thread {}:: Average color sorting took: {:?}",
                    thread_num,
                    avg_colors_duration
                );

                // Send average colors as pixels to WLED
                log::info!("Thread {}:: Sending average colors as pixels", thread_num);
                let send_start = Instant::now();
                let result = arduino::set_pixels("192.168.0.28", avg_colors);
                let send_duration = send_start.elapsed();
                match result {
                    Ok(_) => log::info!(
                        "Average colors set as pixels, sending took: {:?}",
                        send_duration
                    ),
                    Err(e) => log::error!("Error in setting average colors as pixels: {}", e),
                }

                let loop_duration = loop_start.elapsed();
                log::warn!(
                    "Thread {}:: Loop iteration took: {:?}",
                    thread_num,
                    loop_duration
                );

                // // Wait till the allocated time for the loop is over
                // let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
                // let remaining = frame_duration
                //     .checked_sub(loop_duration)
                //     .unwrap_or(Duration::from_secs(0));
                // if remaining.as_secs_f32() > 0.0 {
                //     thread::sleep(remaining);
                // }

                
                // Stop Loop if requested by the UI
                let state = shared_state.lock().unwrap();
                // Log activation/deactivation
                if state.is_active {
                    log::info!("Backend activated");
                } else {
                    drop(state); // Unlock the mutex before sleeping
                    while !shared_state.lock().unwrap().is_active {
                        log::info!("Thread {}:: Backend deactivated", thread_num);
                        // sleep 500ms
                        thread::sleep(Duration::from_millis(500));

                    }
                }
            }
        });
        handles.push(handle);
    }
    handles
}

#[allow(dead_code)]
fn test_arduino() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger (creates a log file)
    // logger::init_logger()?;

    // Run the streaming function
    let result = arduino::set_pixels_red("192.168.0.28");
    match result {
        Ok(_) => log::info!("Color set"),
        Err(e) => log::error!("Error in setting color: {}", e),
    }

    let result = arduino::set_effect("192.168.0.28", 41);
    match result {
        Ok(_) => log::info!("Color set"),
        Err(e) => log::error!("Error in setting color: {}", e),
    }
    Ok(())
}
