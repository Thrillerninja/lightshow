mod screen_capture;
mod arduino;
mod logger;
mod config;
mod hardware_interaction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger (creates a log file)
    logger::init_logger()?;

    // Run the streaming function
    let result = arduino::check_wled_online("192.168.0.28");
    match result {
        Ok(_) => log::info!("Streaming finished"),
        Err(e) => log::error!("Error in streaming: {}", e),
    }

    loop{
      // Get the average colors of the border
      let result = screen_capture::capture_and_process_edge_color(&config::read_config("0current_config.txt")?);

      match result {
        Ok(avg_colors) => {
          // Sort pixels by name
          let mut avg_colors = avg_colors;
          avg_colors.sort_by(|a, b| b.name.cmp(&a.name));

          // Send average colors as pixels
          let result = arduino::set_pixels("192.168.0.28", avg_colors.into());
          match result {
            Ok(_) => log::info!("Average colors set as pixels"),
            Err(e) => log::error!("Error in setting average colors as pixels: {}", e),
          }
        },
        Err(e) => log::error!("Error in capturing and processing: {}", e),
      }

      log::info!("Iteration finished");
    }
    Ok(())
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
