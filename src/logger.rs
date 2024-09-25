use simplelog::*;
use std::fs::File;

pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {

    let config = ConfigBuilder::new()
        .set_time_format_custom(format_description!("[hour]:[minute]:[second].[subsecond digits:3]"))
        .build();

    CombinedLogger::init(vec![
        // Create a log file named "output.log"
        WriteLogger::new(LevelFilter::Info, config, File::create("output.log")?),
    ])?;
    Ok(())
}
