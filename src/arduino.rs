use std::error::Error;

use crate::screen_capture::Color;

// Function to check if WLED is online
pub fn check_wled_online(web_address: &str) -> Result<(), Box<dyn Error>> {
    let url = format!("http://{}/json/state", web_address);

    let client = reqwest::blocking::Client::new();
    let response = client.get(&url).send()?;

    match response.json::<serde_json::Value>()
        .map_err(|e| e.to_string())?
    {
        serde_json::Value::Object(map) => {
            log::info!("WLED is online");
            log::info!("WLED response: {:?}", map);
            Ok(())
        }
        _ => Err("Failed to connect to WLED".into()),
    }
}

// Function to send pixel data to WLED
pub fn set_pixels_red(
    web_address: &str
) -> Result<(), Box<dyn Error>> {
    let url = format!("http://{}/json/state", web_address);

    let client = reqwest::blocking::Client::new();

    let response = client.post(&url)
        .json(&serde_json::json!({
            "on": true,
            "bri": 255,
            "seg": [
                {
                    "start": 0,
                    "stop": 500,
                    "col": [
                        {
                            "r": 255,
                            "g": 0,
                            "b": 0
                        }
                    ]
                }
            ]
        }))
        .send()?;

    match response.json::<serde_json::Value>()
        .map_err(|e| e.to_string())?
    {
        serde_json::Value::Object(map) => {
            log::info!("WLED response: {:?}", map);
            return Ok(())
        }
        _ => return Err("Failed to connect to WLED".into()),
    }
}

pub fn set_effect(
    web_address: &str,
    effect_id: u8
) -> Result<(), Box<dyn Error>> {
    let url = format!("http://{}/json/state", web_address);

    let client = reqwest::blocking::Client::new();

    let response = client.post(&url)
        .json(&serde_json::json!({
            "on": true,
            "bri": 255,
            "seg": [
                {
                    "start": 0,
                    "stop": 500,
                    "fx": effect_id
                }
            ]
        }))
        .send()?;

    match response.json::<serde_json::Value>()
        .map_err(|e| e.to_string())?
    {
        serde_json::Value::Object(map) => {
            log::info!("WLED response: {:?}", map);
            return Ok(())
        }
        _ => return Err("Failed to connect to WLED".into()),
    }
}

pub fn set_pixels(
    web_address: &str,
    pixels: Vec<Color>
) -> Result<(), Box<dyn Error>> {

    let formatted_pixels: Vec<String> = pixels.into_iter()
        .map(|color| color.to_hex())
        .collect();

    log::info!("Formatted pixels: {:?}", formatted_pixels);

    let url = format!("http://{}/json/state", web_address);

    let client = reqwest::blocking::Client::new();

    let response = client.post(&url)
        .json(&serde_json::json!({
            "on": true,
            "bri": 255,
            "seg": [
                {
                    "start": 0,
                    "i": formatted_pixels
                }
            ]
        }))
        .send()?;

    match response.json::<serde_json::Value>()
        .map_err(|e| e.to_string())?
    {
        serde_json::Value::Object(map) => {
            log::info!("WLED response: {:?}", map);
            return Ok(())
        }
        _ => return Err("Failed to connect to WLED".into()),
    }
}