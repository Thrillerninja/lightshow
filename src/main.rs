use std::{collections::HashMap, sync::{Arc, Mutex}, thread, time::{Duration, Instant}};

mod screen_capture;
mod arduino;
mod logger;
mod config;
mod hardware_interaction;
use concurrent_queue::ConcurrentQueue;
use hardware_interaction::{get_monitor_info, MonitorInfo, SlimMonitorInfo};
use image::{Pixel, RgbaImage};
use once_cell::sync::Lazy;
use screen_capture::{calculate_avg_colors, combine_screens, process_edge_color, Color};
use windows_capture::{
  capture::{GraphicsCaptureApiHandler},
  encoder::{AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder},
  frame::Frame,
  graphics_capture_api::InternalCaptureControl,
  monitor::Monitor,
  settings::{ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings},
};
use crate::screen_capture::{save_screenshot_with_avg_colors};
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
  //let processing_handle = process_frames_setup(processing_queue);
  let processing_handle = process_frames_setup_map(monitors.clone().into_iter().map(|m| m.export()).collect());

  // Start capture for each monitor
  let mut capture_handles = Vec::new();
  for (i, monitor_info) in monitors.into_iter().enumerate() {
    // Clone the necessary data before moving it into the thread
    let monitor_handle = Monitor::from_raw_hmonitor(monitor_info.monitor);
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
  }

  // Wait for the processing threads to finish (optional)
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
  let num_threads = 4; // Number of threads for processing
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
        let combined_img = combine_screens(&value, combined_monitor_width as u32, combined_monitor_height as u32, thread_num as u32, min_x, min_y).unwrap();

        let mut avg_colors = calculate_avg_colors(&combined_img, min_x, min_y, max_x, max_y, &CONFIG.leds_array).unwrap();

        match save_screenshot_with_avg_colors(&combined_img, &CONFIG.leds_array, &avg_colors, "combined_img_avg_color.png", min_x, min_y, max_x, max_y)
        {
          Ok(_) => log::info!("Thread {}:: Combined image saved", thread_num),
          Err(e) => log::error!("Thread {}:: Failed to save combined image: {:?}", thread_num, e),
        }

        //  Sort the average colors by LED index
        avg_colors.sort_by(|a, b| a.led_index.cmp(&b.led_index));

        // Send average colors as pixels to WLED
        log::info!("Thread {}:: Sending average colors as pixels:: {:?}", thread_num, avg_colors);
        let result = arduino::set_pixels("192.168.0.28", avg_colors);
        match result {
          Ok(_) => log::info!("Average colors set as pixels"),
          Err(e) => log::error!("Error in setting average colors as pixels: {}", e),
        }

        //if let Err(e) = combined_img.save("combined_img.png") {
        //  log::error!("Thread {}:: Failed to save combined image: {:?}", thread_num, e);
        //} else {
        //  log::info!("Thread {}:: Combined image ready", thread_num);
        //}
      }
    });
    handles.push(handle);
  }
  handles
}






fn process_frames_setup(processing_queue: Arc<ConcurrentQueue<FrameData>>) -> Vec<thread::JoinHandle<()>> {
  let num_threads = 4; // Number of threads for processing
  let mut handles = Vec::with_capacity(num_threads);

  for _ in 0..num_threads {
    let queue_clone = Arc::clone(&processing_queue);
    let handle = thread::spawn(move || {
      let mut i = 0;
      loop {
        // Attempt to dequeue a frame
        if let Ok(frame_data) = queue_clone.pop() {
          log::info!("Frame data: {:?}", queue_clone.len());
          // Convert raw data to image
          if let Some(img) = RgbaImage::from_raw(frame_data.width, frame_data.height, frame_data.data) {
            match process_edge_color(img.clone(), &CONFIG.leds_array) { // Ensure CONFIG is accessible here
              Ok(mut avg_colors) => {
                avg_colors.sort_by(|a, b| a.led_index.cmp(&b.led_index));

                //match arduino::set_pixels("192.168.0.28", avg_colors.into()) {
                //    Ok(_) => log::info!("Average colors set as pixels"),
                //    Err(e) => log::error!("Error setting pixels: {}", e),
                //}

                save_screenshot_with_avg_colors(&img.clone(), &CONFIG.leds_array, &avg_colors, &format!("screenshot_avg_colors{}.png", i), 0, 0, 2560, 1440).unwrap();
              },
              Err(e) => log::error!("Error processing edge color: {}", e),
            }
          } else {
            log::error!("Failed to create image from frame data");
          }
        } else {
          // If no frame is available, sleep briefly to avoid busy waiting
          thread::sleep(Duration::from_millis(10));
        }
        i += 1;
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
