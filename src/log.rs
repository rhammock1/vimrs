pub mod log {
  pub fn log(level: String, message: String) {
    use std::{fs, io::Write};

    let full_message = format!(
      "{} - {}: {}\n",
      chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
      level,
      message,
    );

    let mut file = fs::OpenOptions::new()
      .read(true)
      .append(true)
      .create(true)
      .open("vimrs.log")
      .expect("Unable to open file.");

    file.write_all(full_message.as_bytes()).expect("Unable to write to file.");
  }
}
