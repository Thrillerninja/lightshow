use serde::Deserialize;
use std::fs::{self};
use regex::Regex;

#[allow(non_snake_case, unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct General {
    pub LightpackMode: String,
    pub IsBacklightEnabled: bool,
}

#[allow(non_snake_case, unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct Grab {
    pub Grabber: String,
    pub IsAvgColorsEnabled: bool,
    pub OverBrighten: u8,
    pub IsSendDataOnlyIfColorsChanges: bool,
    pub Slowdown: u8,
    pub LuminosityThreshold: u8,
    pub IsMinimumLuminosityEnabled: bool,
    pub IsDX1011GrabberEnabled: bool,
    pub IsDX9GrabbingEnabled: bool,
    pub IsApplyGammaRampEnabled: bool,
    pub IsApplyColorTemperatureEnabled: bool,
    pub ColorTemperature: u16,
    pub Gamma: f32,
}

#[allow(non_snake_case, unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct MoodLamp {
    pub LiquidMode: bool,
    pub Color: String,
    pub Speed: u8,
    pub Lamp: u8,
}

#[allow(non_snake_case, unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct SoundVisualizer {
    pub Device: u8,
    pub Visualizer: u8,
    pub MinColor: String,
    pub MaxColor: String,
    pub LiquidMode: bool,
    pub LiquidSpeed: u8,
}

#[allow(non_snake_case, unused)]
#[derive(Debug, Deserialize, Copy, Clone)]
pub struct Device {
    pub RefreshDelay: u8,
    pub IsUsbPowerLedDisabled: bool,
    pub Brightness: u8,
    pub BrightnessCap: u8,
    pub Smooth: u8,
    pub Gamma: f64,
    pub ColorDepth: u8,
    pub IsDitheringEnabled: bool,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Copy, Clone)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}


#[allow(unused)]
#[derive(Debug, Deserialize, Copy, Clone)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

#[allow(non_snake_case, unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct LED {
    #[serde(skip)]
    pub index: i32,
    pub IsEnabled: bool,
    pub Position: Position,
    pub Size: Size,
    pub CoefRed: f32,
    pub CoefGreen: f32,
    pub CoefBlue: f32,
}

#[allow(non_snake_case, unused)]
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub General: General,
    pub Grab: Grab,
    pub MoodLamp: MoodLamp,
    pub SoundVisualizer: SoundVisualizer,
    pub Device: Device,
    #[serde(flatten)]
    pub leds: std::collections::HashMap<String, LED>,
    #[serde(skip)]
    pub leds_array: Vec<LED>, // New array form
}
impl Config {
    pub fn convert_leds_to_array(&mut self) {
        self.leds_array = self.leds
            .drain()
            .filter_map(|(key, value)| {
                if value.CoefRed == 1.0 && value.CoefGreen == 1.0 && value.CoefBlue == 1.0 {
                    None
                } else {
                    Some(LED {
                        index: key.trim_start_matches("LED_").parse().unwrap_or(self.leds_array.len() as i32),
                        IsEnabled: value.IsEnabled,
                        Position: value.Position,
                        Size: value.Size,
                        CoefRed: value.CoefRed,
                        CoefGreen: value.CoefGreen,
                        CoefBlue: value.CoefBlue,
                    })
                }
            })
            .collect();
    }
}

// This function converts the input config to valid TOML format
fn convert_to_toml(input: &str) -> String {
    let mut result = String::new();

    // Regex patterns to match sections, colors, points, sizes, and types
    let section_re = Regex::new(r"^\[(\w+)\]$").unwrap();
    let color_re = Regex::new(r"^([A-Za-z0-9_]+)=#([A-Fa-f0-9]{6})$").unwrap();    
    let point_re = Regex::new(r"^([A-Za-z0-9_]+)\s*=\s*@Point\((-?\d+)\s+(-?\d+)\)$").unwrap();
    let size_re = Regex::new(r"^([A-Za-z0-9_]+)=@Size\((\d+)\s+(\d+)\)$").unwrap();
    let bool_re = Regex::new(r"^(Is)([A-Za-z0-9_]+)=(true|false)$").unwrap();
    let bool_re2 = Regex::new(r"^(LiquidMode)=(true|false)$").unwrap(); // Failure in creating actual standard \_(ツ)_/¯
    let int_re = Regex::new(r"^([A-Za-z0-9_]+)=(\d+)$").unwrap();
    let float_re = Regex::new(r"^([A-Za-z0-9_]+)=([\d.]+)$").unwrap();
    let unquoted_string_re = Regex::new(r"^([A-Za-z0-9_]+)=(\w+)$").unwrap();

    for line in input.lines() {        
        if line.trim().is_empty() {
            continue; // Skip empty lines
        }
        if let Some(caps) = section_re.captures(line) {
            result.push_str(&format!("[{}]\n", &caps[1]));
        } else if let Some(caps) = color_re.captures(line) {
            result.push_str(&format!("{} = \"#{}\"\n", &caps[1], &caps[2]));
        } else if let Some(caps) = point_re.captures(line) {
            result.push_str(&format!("{} = {{ x = {}, y = {} }}\n", &caps[1], &caps[2], &caps[3]));
        } else if let Some(caps) = size_re.captures(line) {
            result.push_str(&format!("{} = {{ width = {}, height = {} }}\n", &caps[1], &caps[2], &caps[3]));
        } else if let Some(caps) = bool_re.captures(line) {
            result.push_str(&format!("Is{} = {}\n", &caps[2], &caps[3]));
        } else if let Some(caps) = bool_re2.captures(line) {
            result.push_str(&format!("{} = {}\n", &caps[1], &caps[2]));
        } else if let Some(caps) = int_re.captures(line) {
            result.push_str(&format!("{} = {}\n", &caps[1], &caps[2]));
        } else if let Some(caps) = float_re.captures(line) {
            result.push_str(&format!("{} = {}\n", &caps[1], &caps[2]));
        } else if let Some(caps) = unquoted_string_re.captures(line) {
            result.push_str(&format!("{} = \"{}\"\n", &caps[1], &caps[2]));
        } else {
            // Log or handle unmatched lines
            eprintln!("Unmatched line: {}", line);
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

pub fn read_config(file_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_content = fs::read_to_string(file_path)?;
    let fixed_config_content = convert_to_toml(&config_content);
    let mut config: Config = toml::from_str(&fixed_config_content)?;

    // Convert the HashMap to a Vec to enable parallel processing
    config.convert_leds_to_array();

    log::info!("Config loaded");
    Ok(config)
}