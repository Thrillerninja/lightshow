mod screen_capture;
mod arduino;
mod logger;
mod config;
mod hardware_interaction;

use screen_capture::Color;

use reqwest::{Error, Response};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger (creates a log file)
    logger::init_logger()?;

    // Run the streaming function
    let result = arduino::check_wled_online("192.168.0.28");
    match result {
        Ok(_) => log::info!("Streaming finished"),
        Err(e) => log::error!("Error in streaming: {}", e),
    }

    let result = arduino::set_pixels("192.168.0.28", [Color::new(255, 0, 0), Color::new(255, 0, 0), Color::new(255, 0, 0), Color::new(0, 255, 0), Color::new(0, 255, 0), Color::new(0, 255, 0), Color::new(0, 0, 255), Color::new(0, 0, 255), Color::new(0, 0, 255)].into());
    match result {
        Ok(_) => log::info!("Color set"),
        Err(e) => log::error!("Error in setting color: {}", e),
    }
    

    log::info!("Iteration finished");
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
