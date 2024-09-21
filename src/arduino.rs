use serialport;
use std::time::Duration;
use std::io::Write;
use crate::screen_capture::Color;

// Function to send the average color to the Arduino
pub fn send_color_to_arduino(color_stripe: Vec<Color>) -> Result<(), Box<dyn std::error::Error>> {
    // Open the serial port for communication with the Arduino
    let mut port = serialport::new("COM5", 9600)
        .timeout(Duration::from_secs(1))
        .open()?;

    // Send the color in the format "R,G,B\n"
    let color_data = color_stripe.iter().map(|color| {
        format!("{},{},{}\n", color.r, color.g, color.b)
    }).collect::<Vec<String>>().join("");

    //port.write(color_data.as_bytes())?;

    println!("Sent color update to Arduino");
    log::info!("Sent color update to Arduino");
    Ok(())
}
