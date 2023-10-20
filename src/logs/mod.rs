use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
};

use env_logger::fmt::Color;
use log::LevelFilter;

use crate::{config::LoggingSettings, utils::get_path};

use self::dual_writer::DualWriter;

mod dual_writer;

pub fn get_log_path(settings: &LoggingSettings) -> Option<PathBuf> {
    if settings.save_as == "file" {
        Some(get_path(&settings.path))
    } else if settings.save_as == "dir" {
        let yyyy_mm_dd = chrono::Local::now().format("%Y-%m-%d");
        let mut log_file = 1;

        // Check if log file already exists.
        while get_path(&settings.path)
            .join(format!("{}-{}.log", yyyy_mm_dd, log_file))
            .exists()
        {
            log_file += 1;
        }

        Some(get_path(&settings.path).join(format!("{}-{}.log", yyyy_mm_dd, log_file)))
    } else {
        None
    }
}

pub fn get_log_file(settings: &LoggingSettings) -> Option<Box<File>> {
    let file_path = get_log_path(settings);

    if file_path != None {
        let file_path = file_path.unwrap();
        let file = File::create(file_path).unwrap();
        Some(Box::new(file))
    } else {
        None
    }
}

pub fn stdout_target() -> Box<dyn io::Write + Send> {
    Box::new(io::stdout())
}

pub fn setup_logger(settings: &LoggingSettings) {
    let mut logger = env_logger::Builder::new();
    logger
        .filter_level(LevelFilter::Info)
        .format(|buf, record| {
            // Time color.
            let mut time_style = buf.style();
            time_style.set_color(Color::Cyan);

            // Get color for level.
            let level_style = buf.default_level_style(record.level());

            // Get color for target.
            let mut target_style = buf.style();
            target_style.set_color(Color::Magenta);

            writeln!(
                buf,
                "[{}] {} {} - {}",
                time_style.value(chrono::Local::now().format("%Y-%m-%d %H:%M:%S")),
                level_style.value(record.level()),
                target_style.value(record.target()),
                record.args()
            )
        });

    // Check if we should save logs to a file.
    let file = get_log_file(&settings);
    if file.is_some() {
        let file = file.unwrap();
        let pipe: Box<dyn io::Write + Send> = DualWriter::new(file, stdout_target()).into();
        logger.target(env_logger::Target::Pipe(pipe)).init();
    } else {
        logger.init();
    }
}
