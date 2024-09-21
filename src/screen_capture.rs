use image::{ImageBuffer, RgbaImage};
use std::time::Duration;
use std::{fs::File, thread};
use std::path::Path;
use scrap::{Capturer, Display};

// Define the Color struct
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn capture_and_process_edge_color() -> Result<Vec<Color>, Box<dyn std::error::Error>> {
    // Assuming screens is a Vec of Capturer objects
    let mut screens: Vec<Capturer> = Vec::new();

    for display in Display::all()?.into_iter() {
        screens.push(Capturer::new(display)?);
    }

    // Check if screens vector is empty
    if screens.is_empty() {
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No screens found")));
    }

    // Get the dimensions of each screen
    let screen_dimensions: Vec<(u32, u32)> = screens.iter().map(|screen| (screen.width() as u32, screen.height() as u32)).collect();

    // Calculate the total width and height for the combined screenshot
    let total_width = screen_dimensions.iter().map(|(width, _)| width).sum();
    let total_height: u32 = *screen_dimensions.iter().map(|(_, height)| height).max().unwrap_or(&1u32);

    // Create an image buffer for the combined screenshot
    let mut combined_image: RgbaImage = ImageBuffer::new(total_width, total_height);

    let mut x_offset = 0;
    for (screen, (width, height)) in screens.iter_mut().zip(screen_dimensions.iter()) {
        let screenshot = loop {
            match screen.frame() {
                Ok(frame) => break frame,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Frame not ready yet, wait for a short period and retry.
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(e) => return Err(Box::new(e)),
            }
        };
        let stride = screenshot.len() / (*height as usize);

        // Copy the screenshot into the combined image buffer
        for (y, row) in screenshot.chunks(stride).enumerate() {
            for (x, pixel) in row.chunks(4).enumerate() {
                let b = pixel[0];
                let g = pixel[1];
                let r = pixel[2];
                let a = 255u8; // Opaque alpha value

                combined_image.put_pixel((x_offset + x) as u32, y as u32, image::Rgba([r, g, b, a]));
            }
        }

        x_offset += *width as usize;
    }

    // Save the combined screenshot to a file
    let path = Path::new("screenshot.png");
    let mut file = File::create(path)?;
    image::DynamicImage::ImageRgba8(combined_image).write_to(&mut file, image::ImageFormat::Png)?;

    println!("Screenshot saved to screenshot.png");

    // Calculate the average color from the edges and a 20x20 box
    let avg_color: Vec<Color> = vec![Color { r: 255, g: 128, b: 0 }]; //calculate_average_color(&combined_image.clone())?;

    Ok(avg_color)
}

// Function to calculate average color from edges and a 20x20 box
fn calculate_average_color(image: &RgbaImage) -> Result<Color, Box<dyn std::error::Error>> {
    let (width, height) = image.dimensions();
    let mut total_r = 0u64;
    let mut total_g = 0u64;
    let mut total_b = 0u64;
    let mut count = 0u64;

    // Top and bottom edges
    for x in 0..width {
        for y in [0, height - 1].iter() {
            let pixel = image.get_pixel(x, *y);
            total_r += pixel[0] as u64;
            total_g += pixel[1] as u64;
            total_b += pixel[2] as u64;
            count += 1;
        }
    }

    // Left and right edges
    for y in 1..height - 1 {
        for x in [0, width - 1].iter() {
            let pixel = image.get_pixel(*x, y);
            total_r += pixel[0] as u64;
            total_g += pixel[1] as u64;
            total_b += pixel[2] as u64;
            count += 1;
        }
    }

    // 20x20 box
    for y in 0..20.min(height) {
        for x in 0..20.min(width) {
            let pixel = image.get_pixel(x, y);
            total_r += pixel[0] as u64;
            total_g += pixel[1] as u64;
            total_b += pixel[2] as u64;
            count += 1;
        }
    }

    for y in (height - 20).max(0)..height {
        for x in (width - 20).max(0)..width {
            let pixel = image.get_pixel(x, y);
            total_r += pixel[0] as u64;
            total_g += pixel[1] as u64;
            total_b += pixel[2] as u64;
            count += 1;
        }
    }

    // Calculate average color
    let avg_r = (total_r / count) as u8;
    let avg_g = (total_g / count) as u8;
    let avg_b = (total_b / count) as u8;

    Ok(Color { r: avg_r, g: avg_g, b: avg_b })
}