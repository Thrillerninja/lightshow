use image::{Rgba, RgbaImage};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::hardware_interaction::capture_screenshot;

// Define the Color struct
#[derive(Debug, Clone)]
pub struct Color {
    pub led_index: i32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Color {

    pub fn new(led_index: i32, r: u8, g: u8, b: u8) -> Self {
        Color { led_index, r, g, b }
    }

    pub fn to_hex(&self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

}

fn save_screenshot(image: &RgbaImage, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path);
    let mut file = File::create(path)?;
    image::DynamicImage::ImageRgba8(image.clone()).write_to(&mut file, image::ImageFormat::Png)?;
    Ok(())
}


pub fn capture_and_process_edge_color(config: &Config) -> Result<Vec<Color>, Box<dyn std::error::Error>> {
    let (screenshot_img, min_x, min_y, max_x, max_y) = capture_screenshot()?;
    //save_screenshot(&screenshot_img, "screenshot.png")?;
//
    //log::info!("Border started");
    //if let Err(e) = save_config_border_img(&screenshot_img, min_x, min_y, max_x, max_y, config) {
    //    log::error!("Failed to save border image: {}", e);
    //} else {
    //    log::info!("Border image saved");
    //}

    let avg_colors = calculate_avg_colors(&screenshot_img, min_x, min_y, max_x, max_y, config)?;
    log::info!("Average colors calculated");

    //save_screenshot_with_avg_colors(&screenshot_img, config, &avg_colors, "screenshot_avg_colors.png", min_x, min_y, max_x, max_y)?;
    
    Ok(avg_colors)
}

fn save_config_border_img(
    screenshot_img: &RgbaImage, 
    min_x: i32, 
    min_y: i32, 
    max_x: i32, 
    max_y: i32, 
    config: &Config
) -> Result<(), Box<dyn std::error::Error>> {
    let screenshot = Arc::new(Mutex::new(screenshot_img.clone()));

    config.leds_array.par_iter().for_each(|led| {
        let position = (led.Position.x, led.Position.y);
        let size = (led.Size.width, led.Size.height);

        for x in 0..size.0 {
            for y in 0..size.1 {
                // Calculate pixel positions relative to the screen and clamp to valid image area
                let pixel_x = (position.0 + x - min_x) as i32;
                let pixel_y = (position.1 + y - min_y) as i32;

                // Skip out-of-bounds pixels entirely
                if pixel_x < 0 || pixel_y < 0 || pixel_x >= max_x as i32 || pixel_y >= max_y as i32 {
                    continue;
                }

                // Only draw the border (outline)
                if x == 0 || x == size.0 - 1 || y == 0 || y == size.1 - 1 {
                    let mut screenshot = screenshot.lock().unwrap();
                    screenshot.put_pixel(
                        pixel_x as u32,
                        pixel_y as u32,
                        image::Rgba([255, 0, 0, 255]),
                    );
                }
            }
        }
    });

    // Unwrapping the screenshot Arc after drawing
    let screenshot_out = Arc::try_unwrap(screenshot)
        .expect("Screenshot still has multiple references")
        .into_inner()
        .unwrap();

    // Saving the modified screenshot
    save_screenshot(&screenshot_out, "screenshot_border.png")?;
    Ok(())
}


fn calculate_avg_colors(image: &RgbaImage, min_x: i32, min_y: i32, max_x: i32, max_y: i32, config: &Config) -> Result<Vec<Color>, Box<dyn std::error::Error>> {

    let avg_colors: Vec<Color> = config.leds_array.par_iter().map(|led| {        
        let mut r_sum = 0;
        let mut g_sum = 0;
        let mut b_sum = 0;
        let mut count = 0;

        let position = (led.Position.x, led.Position.y);
        let size = (led.Size.width, led.Size.height);
        for x in 0..size.0 {
            for y in 0..size.1 {
                // Calculate pixel positions relative to the screen and clamp to valid image area
                let pixel_x = (position.0 + x - min_x) as i32;
                let pixel_y = (position.1 + y - min_y) as i32;

                // Skip out-of-bounds pixels entirely
                if pixel_x < 0 || pixel_y < 0 || pixel_x >= max_x as i32 || pixel_y >= max_y as i32 {
                    continue;
                }

                let pixel = image.get_pixel(
                        pixel_x as u32,
                        pixel_y as u32
                    );
                r_sum += pixel[0] as u32;
                g_sum += pixel[1] as u32;
                b_sum += pixel[2] as u32;
                count += 1;
            }
        }
        
        if count != 0 {
            Color::new(
                led.index.clone(),
                ((r_sum / count) as f32) as u8, // * (1./led.CoefRed)
                ((g_sum / count) as f32) as u8, // * (1./led.CoefGreen)
                ((b_sum / count) as f32) as u8) // * (1./led.CoefBlue)
        } else {
            Color::new(led.index.clone(), 0, 0, 0) // Default to black if no pixels are counted
        }
        
    }).collect();
    
    Ok(avg_colors)
}


fn save_screenshot_with_avg_colors(
    image: &RgbaImage,
    config: &Config,
    avg_colors: &Vec<Color>,
    path: &str,
    min_x: i32, min_y: i32, max_x: i32, max_y: i32
) -> Result<(), Box<dyn std::error::Error>> {
    //Create a clone of the image to store results, and we'll merge them later
    let result_image = Arc::new(Mutex::new(image.clone()));
    
    log::info!("Saving screenshot with average colors started");

    config.leds_array.clone().par_iter().enumerate().for_each(|(i,led)| {
        let position = (led.Position.x, led.Position.y);
        let size = (led.Size.width, led.Size.height);
        let color = Color::new(led.index.clone(), avg_colors[i].r, avg_colors[i].g, avg_colors[i].b);

        for x in 0..size.0 {
            for y in 0..size.1 {
                // Calculate pixel positions relative to the screen and clamp to valid image area
                let pixel_x = (position.0 + x - min_x) as i32;
                let pixel_y = (position.1 + y - min_y) as i32;

                // Skip out-of-bounds pixels entirely
                if pixel_x < 0 || pixel_y < 0 || pixel_x >= max_x as i32 || pixel_y >= max_y as i32 {
                    continue;
                }

                // Directly modify the result image instead of locking the main one
                let mut result_image = result_image.lock().unwrap();
                result_image.put_pixel(
                    pixel_x as u32,
                    pixel_y as u32,
                    Rgba([color.r, color.g, color.b, 255]),
                );
            }
        }
    });

    let screenshot_out = Arc::try_unwrap(result_image).expect("Screenshot still has multiple references").into_inner().unwrap();

    log::info!("Saving screenshot with average colors");
    save_screenshot(&screenshot_out, path)?;

    Ok(())
}
