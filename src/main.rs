use std::{collections::HashMap, sync::{Arc, Mutex}, thread, time::{Duration, Instant}};
mod screen_capture;
mod arduino;
mod logger;
mod config;
mod hardware_interaction;
use concurrent_queue::ConcurrentQueue;
use hardware_interaction::{get_monitor_info, SlimMonitorInfo};
use once_cell::sync::Lazy;
use screen_capture::{calculate_avg_colors, combine_screens, save_screenshot_with_avg_colors};
use windows_capture::{
  capture::GraphicsCaptureApiHandler,
  monitor::Monitor,
  settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
};
use crate::hardware_interaction::{FrameData, Capture};

// Define a global, thread-safe queue
static FRAME_QUEUE: Lazy<Arc<ConcurrentQueue<FrameData>>> = Lazy::new(|| {
  Arc::new(ConcurrentQueue::unbounded())
});

static FRAME_MAP: Lazy<Arc<Mutex<HashMap<i32, FrameData>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

static CONFIG: Lazy<config::Config> = Lazy::new(|| {
  config::read_config("0current_config.txt").expect("Failed to read config file")
});

fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logging (optional)
  logger::init_logger()?;

  // Clone the queue for the processing thread
  //let processing_queue = Arc::clone(&FRAME_QUEUE);
  //let processing_map = Arc::clone(&FRAME_MAP);

  // Retrieve monitor information (if needed)
  let monitors = get_monitor_info()?;
  println!("Monitors: {:?}", monitors);

  // Start the processing thread
  let processing_start = Instant::now();
  let processing_handle = process_frames_setup_map(monitors.clone().into_iter().map(|m| m.export()).collect());
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
                DrawBorderSettings::Default,
                ColorFormat::Rgba8,
                i.to_string(),
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
        println!("Capture thread setup for monitor {} took: {:?}", i, capture_duration);
    }

    // Wait for the capture threads to finish (optional)
    for handle in capture_handles {
        handle.join().unwrap();
    }

    // Wait for the processing threads to finish (optional)
    for handle in processing_handle {
        handle.join().unwrap();
    }

    Ok(())
}

fn process_frames_setup_map(monitors: Vec<SlimMonitorInfo>) -> Vec<thread::JoinHandle<()>> {
  let num_threads = 1; // Number of threads for processing
  let mut handles = Vec::with_capacity(num_threads);

  let combined_monitor_width: i32 = monitors.iter().map(|m| m.width).sum();
  let combined_monitor_height: i32 = monitors.iter().map(|m| m.height).max().unwrap();

  for thread_num in 0..num_threads {
      let value: Vec<SlimMonitorInfo> = monitors.clone();
      let handle = thread::spawn(move || {
          let min_x = value.iter().map(|mi| mi.pos_x).min().unwrap_or(0);
          let min_y = value.iter().map(|mi| mi.pos_y).min().unwrap_or(0);
          let max_x = value.iter().map(|mi| mi.pos_x + mi.width).max().unwrap_or(0);
          let max_y = value.iter().map(|mi| mi.pos_y + mi.height).max().unwrap_or(0);
          log::info!("Combined Screen dimensions:: min_x: {}, min_y: {}, max_x: {}, max_y: {}", min_x, min_y, max_x, max_y);

          loop {
              let loop_start = Instant::now(); // Start timing the loop

              let combined_img_start = Instant::now();
              let combined_img = combine_screens(&value, combined_monitor_width as u32, combined_monitor_height as u32, thread_num as u32, min_x, min_y).unwrap();
              log::info!("Thread {}:: Combined image creation took: {:?}", thread_num, combined_img_start.elapsed());

              let avg_colors_start = Instant::now();
              let mut avg_colors = calculate_avg_colors(&combined_img, min_x, min_y, max_x, max_y, &CONFIG.leds_array).unwrap();
              let avg_colors_duration = avg_colors_start.elapsed();
              log::info!("Thread {}:: Average color calculation took: {:?}", thread_num, avg_colors_duration);

              //let save_start = Instant::now();
              //match save_screenshot_with_avg_colors(&combined_img, &CONFIG.leds_array, &avg_colors, "combined_img_avg_color.png", min_x, min_y, max_x, max_y) {
              //    Ok(_) => log::info!("Thread {}:: Combined image saved", thread_num),
              //    Err(e) => log::error!("Thread {}:: Failed to save combined image: {:?}", thread_num, e),
              //}
              //let save_duration = save_start.elapsed();
              //log::info!("Thread {}:: Save operation took: {:?}", thread_num, save_duration);

              // Sort the average colors by LED index
              let avg_colors_start = Instant::now();
              avg_colors.sort_by(|a, b| a.led_index.cmp(&b.led_index));
              let avg_colors_duration = avg_colors_start.elapsed();
              log::info!("Thread {}:: Average color sorting took: {:?}", thread_num, avg_colors_duration);

              // Send average colors as pixels to WLED
              log::info!("Thread {}:: Sending average colors as pixels", thread_num);
              let send_start = Instant::now();
              let result = arduino::set_pixels("192.168.0.28", avg_colors);
              let send_duration = send_start.elapsed();
              match result {
                  Ok(_) => log::info!("Average colors set as pixels, sending took: {:?}", send_duration),
                  Err(e) => log::error!("Error in setting average colors as pixels: {}", e),
              }

              let loop_duration = loop_start.elapsed();
              log::info!("Thread {}:: Loop iteration took: {:?}", thread_num, loop_duration);

              // Sleep briefly to avoid high CPU usage
              //thread::sleep(Duration::from_millis(1000));
          }
      });
      handles.push(handle);
  }
  handles
}



#[allow(dead_code)]
fn test_arduino() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger (creates a log file)
    logger::init_logger()?;

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
