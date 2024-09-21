use simplelog::*;
use std::fs::File;

pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
    CombinedLogger::init(vec![
        // Create a log file named "output.log"
        WriteLogger::new(LevelFilter::Info, Config::default(), File::create("output.log")?),
    ])?;
    Ok(())
}
