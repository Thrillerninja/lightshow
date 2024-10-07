use image::{GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::ptr::copy_nonoverlapping;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::config::{Config, LED};
use crate::hardware_interaction::{FrameData, SlimMonitorInfo};
use crate::FRAME_MAP;

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


pub fn process_edge_color(screenshot_img: image::ImageBuffer<Rgba<u8>, Vec<u8>>, leds_array: &Vec<LED> ) -> Result<Vec<Color>, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();
    save_screenshot(&screenshot_img, "screenshot.png")?;

    //log::info!("Border started");
    //if let Err(e) = save_config_border_img(&screenshot_img, min_x, min_y, max_x, max_y, config) {
    //    log::error!("Failed to save border image: {}", e);
    //} else {
    //    log::info!("Border image saved");
    //}

    let avg_colors = calculate_avg_colors(&screenshot_img, 0, 0, 1000, 1000, leds_array)?;
    log::info!("Average colors calculated");

    //save_screenshot_with_avg_colors(&screenshot_img, config, &avg_colors, "screenshot_avg_colors.png", min_x, min_y, max_x, max_y)?;
    
    Ok(avg_colors)
}

pub fn save_config_border_img(
    screenshot_img: &RgbaImage, 
    min_x: i32, 
    min_y: i32, 
    max_x: i32, 
    max_y: i32, 
    leds_array: &Vec<LED>
) -> Result<(), Box<dyn std::error::Error>> {
    let screenshot = Arc::new(Mutex::new(screenshot_img.clone()));

    leds_array.par_iter().for_each(|led| {
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

pub fn combine_screens(value: &Vec<SlimMonitorInfo>, combined_monitor_width: u32, combined_monitor_height: u32, thread_num: u32, min_x: i32, min_y: i32) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    let mut combined_img: ImageBuffer<Rgba<u8>, Vec<u8>> = RgbaImage::new(combined_monitor_width, combined_monitor_height);

    // Lock the map briefly to copy the frame data, then release the lock
    let frame_data_copy: HashMap<i32, FrameData> = {
        let frame_map = FRAME_MAP.lock().unwrap();
        frame_map.clone() // Clone the map contents
    };
    //log::info!("Thread {}:: Frame data copy took: {:?}", thread_num, start_time.elapsed());

    // Process the copied frame data
    for (i, monitor) in value.iter().enumerate() {
        if let Some(frame_data) = frame_data_copy.get(&(i as i32)) {
            let position: (i32, i32) = (monitor.pos_x, monitor.pos_y);

            // Ensure the subtraction does not result in a negative value
            let x_offset = (position.0 - min_x).max(0) as u32;
            let y_offset = (position.1 - min_y).max(0) as u32;

            let img_width = monitor.width as u32;
            let img_height = monitor.height as u32;

            // Direct buffer copy using copy_from_slice
            for y in 0..img_height {
                let src_start = (y * img_width * 4) as usize;
                let src_end = src_start + (img_width * 4) as usize;
                let dest_start = ((y_offset + y) * combined_monitor_width * 4 + x_offset * 4) as usize;

                unsafe {
                    let src_ptr = frame_data.data.as_ptr().add(src_start);
                    let dest_ptr = combined_img.as_mut_ptr().add(dest_start);
                    copy_nonoverlapping(src_ptr, dest_ptr, src_end - src_start);
                }
            }

            //log::info!("Thread {}:: Image {} copied successfully in {:?}", thread_num, i, start_time.elapsed());
        }
    }

    log::info!("Thread {}:: Combined image creation took: {:?}", thread_num, start_time.elapsed());
    Ok(combined_img)
}

pub fn calculate_avg_colors(image: &RgbaImage, min_x: i32, min_y: i32, max_x: i32, max_y: i32, leds_array: &Vec<LED>) -> Result<Vec<Color>, Box<dyn std::error::Error>> {

    let avg_colors: Vec<Color> = leds_array.par_iter().map(|led| {        
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


pub fn save_screenshot_with_avg_colors(
    image: &RgbaImage,
    leds_array: &Vec<LED>,
    avg_colors: &Vec<Color>,
    path: &str,
    min_x: i32, min_y: i32, max_x: i32, max_y: i32
) -> Result<(), Box<dyn std::error::Error>> {
    //Create a clone of the image to store results, and we'll merge them later
    let result_image = Arc::new(Mutex::new(image.clone()));
    
    log::info!("Saving screenshot with average colors started");

    leds_array.clone().par_iter().enumerate().for_each(|(i,led)| {
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
