use image::RgbaImage;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::hardware_interaction::capture_screenshot;

// Define the Color struct
#[derive(Debug, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
impl Color {

    pub fn new(r: u8, g: u8, b: u8) -> Self {

        Color { r, g, b }

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
    save_screenshot(&screenshot_img, "screenshot.png")?;

    log::info!("Border started");
    match save_config_border_img(&screenshot_img, min_x, min_y, max_x, max_y, config) {
        Ok(_) => log::info!("Border image saved"),
        Err(e) => log::error!("Failed to save border image: {}", e),
    }

    match calculate_avg_colors(&screenshot_img, min_x, min_y, max_x, max_y, config) {
        Ok(avg_colors) => {
            log::info!("Average colors calculated");
            save_screenshot_with_avg_colors(&screenshot_img, config, &avg_colors, "screenshot_avg_colors.png")?;
            return Ok(avg_colors)
        }
        Err(e) => {
            log::error!("Failed to calculate average colors: {}", e);
            return Err(e)
        }
    }
}

fn save_config_border_img(screenshot_img: &RgbaImage, min_x: i32, min_y: i32, max_x: i32, max_y: i32, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let screenshot = Arc::new(Mutex::new(screenshot_img.clone()));

    config.leds_array.par_iter().for_each(|led| {
        // Draw a border around the LED
        let position = (led.Position.x, led.Position.y);
        let size = (led.Size.width, led.Size.height);
        for x in 0..size.0 {
            for y in 0..size.1 {
                if x == 0 || x == size.0 - 1 || y == 0 || y == size.1 - 1 {
                    if (position.0 + x - min_x) < 0 || (position.1 + y - min_y) < 0 {
                        //log::warn!("Negative pixel position in LED configuration: {} ({}, {})", led.name, position.0 + x - min_x, position.1 + y - min_y);
                    } else if (position.0 + x - min_x) > max_x || (position.1 + y - min_y) > max_y {
                        //log::warn!("To large pixel position in LED configuration: {} ({}, {})", led.name, position.0 + x - min_x, position.1 + y - min_y);
                    } else {                        
                        let mut screenshot = screenshot.lock().unwrap();
                        screenshot.put_pixel(
                            (position.0 + x - min_x) as u32,
                            (position.1 + y - min_y) as u32,
                            image::Rgba([255, 0, 0, 255]),
                        );                        
                    }
                }
            }
        }
    });

    let screenshot_out = Arc::try_unwrap(screenshot).expect("Screenshot still has multiple references").into_inner().unwrap();

    save_screenshot(&screenshot_out, "screenshot_border.png")?;
    Ok(())
}

fn calculate_avg_colors(image: &RgbaImage, min_x: i32, min_y: i32, max_x: i32, max_y: i32, config: &Config) -> Result<Vec<Color>, Box<dyn std::error::Error>> {

    let mut avg_colors = Arc::new(Mutex::new(Vec::new()));

    config.leds_array.par_iter().for_each(|led| {
        let mut r_sum = 0;
        let mut g_sum = 0;
        let mut b_sum = 0;
        let mut count = 0;

        let position = (led.Position.x, led.Position.y);
        let size = (led.Size.width, led.Size.height);
        for x in 0..size.0 {
            for y in 0..size.1 {
                if x == 0 || x == size.0 - 1 || y == 0 || y == size.1 - 1 {
                    if (position.0 + x - min_x) < 0 || (position.1 + y - min_y) < 0 {
                        //log::warn!("Negative pixel position in LED configuration: {} ({}, {})", led.name, position.0 + x - min_x, position.1 + y - min_y);
                    } else if (position.0 + x - min_x) > max_x || (position.1 + y - min_y) > max_y {
                        //log::warn!("To large pixel position in LED configuration: {} ({}, {})", led.name, position.0 + x - min_x, position.1 + y - min_y);
                    } else {
                        let pixel = image.get_pixel((position.0 + x - min_x) as u32, (position.1 + y - min_y) as u32);
                        r_sum += pixel[0] as u32;
                        g_sum += pixel[1] as u32;
                        b_sum += pixel[2] as u32;
                        count += 1;
                    }
                }
            }
        }

        let mut r_avg = 0;
        let mut g_avg = 0;
        let mut b_avg = 0;

        if count != 0 {
            r_avg = (r_sum / count) as u8;
            g_avg = (g_sum / count) as u8;
            b_avg = (b_sum / count) as u8;
        } 

        let mut avg_colors = avg_colors.lock().unwrap();
        avg_colors.push(Color { r: r_avg, g: g_avg, b: b_avg });
    });

    let avg_colors = Arc::try_unwrap(avg_colors).expect("Average colors still has multiple references").into_inner().unwrap();

    Ok(avg_colors)
}

fn save_screenshot_with_avg_colors(
    image: &RgbaImage,
    config: &Config,
    avg_colors: &Vec<Color>,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a clone of the image to store results, and we'll merge them later
    // let mut result_image = image.clone();
    
    // log::info!("Saving screenshot with average colors started");

    // config.leds_array.clone().into_par_iter().enumerate().for_each(|(_i, led)| {
    //     let position = (led.Position.x, led.Position.y);
    //     let size = (led.Size.width, led.Size.height);
    //     let color = avg_colors.iter().find(|&c| c.r == 0 && c.g == 0 && c.b == 0).unwrap();

    //     for x in 0..size.0 {
    //         for y in 0..size.1 {
    //             if (position.0 + x) < 0 || (position.1 + y) < 0 {
    //                 // Skip pixels with negative positions
    //             } else if (position.0 + x) >= image.width() as i32 || (position.1 + y) >= image.height() as i32 {
    //                 // Skip pixels that are out of bounds
    //             } else {
    //                 // Directly modify the result image instead of locking the main one
    //                 result_image.put_pixel(
    //                     (position.0 + x) as u32,
    //                     (position.1 + y) as u32,
    //                     Rgba([color.r, color.g, color.b, 255]),
    //                 );
    //             }
    //         }
    //     }
    // });

    // // Now, merge `result_image` into the original `image`
    // let mut screenshot = image.clone();
    // for (x, y, pixel) in result_image.enumerate_pixels() {
    //     // Replace the pixel only if it's modified (non-zero alpha)
    //     if pixel[3] > 0 {
    //         screenshot.put_pixel(x, y, *pixel);
    //     }
    // }

    // log::info!("Saving screenshot with average colors");
    // save_screenshot(&screenshot, path)?;

    Ok(())
}
