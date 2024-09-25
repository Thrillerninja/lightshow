use std::time::Instant;

mod screen_capture;
mod arduino;
mod logger;
mod config;
mod hardware_interaction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger (creates a log file)
    logger::init_logger()?;

    // Check if WLED is online
    let result = arduino::check_wled_online("192.168.0.28");
    match result {
        Ok(_) => log::info!("WLED is online, beginning streaming"),
        Err(e) => log::error!("Error in checking if WLED is online: {}", e),
    }

    let config = match config::read_config("0current_config.txt") {
        Ok(config) => config,
        Err(e) => {
            log::error!("Error reading config: {}", e);
            return Ok(());
        }
    };

    loop{
      
      let start = Instant::now();
      // Get the average colors of the border
      let result = screen_capture::capture_and_process_edge_color(&config);

      match result {
        Ok(avg_colors) => {
          // Sort pixels by name
          let mut avg_colors = avg_colors;
            avg_colors.sort_by(|a, b| {a.led_index.cmp(&b.led_index)});

          // Send average colors as pixels to WLED
          let result = arduino::set_pixels("192.168.0.28", avg_colors.into());
          match result {
            Ok(_) => log::info!("Average colors set as pixels"),
            Err(e) => log::error!("Error in setting average colors as pixels: {}", e),
          }
        },
        Err(e) => log::error!("Error in capturing and processing: {}", e),
      }

      let duration = start.elapsed();
      log::info!("Iteration finished in {:?}", duration);
    }
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
