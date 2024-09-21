mod screen_capture;
mod arduino;
mod logger;

use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger (creates a log file)
    logger::init_logger()?;

    loop {
        // Capture screen and get the processed average color along the edge
        let avg_color = screen_capture::capture_and_process_edge_color()?;

        // Send the average color to the Arduino
        //arduino::send_color_to_arduino(avg_color)?;

        // Limit to 20 FPS
        thread::sleep(Duration::from_millis(50));
    }
}
