use std::{fs, io::Write};

pub fn log(s: &str) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("log.txt")
        .unwrap();
    let out: String = s.to_string() + "\n";
    file.write_all(out.as_bytes()).unwrap();
}
