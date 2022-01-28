use std::io::Write;
use tokio::{fs, io::AsyncWriteExt};

pub async fn log_async(s: &str) {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("log.txt")
        .await
        .unwrap();
    let out: String = s.to_string() + "\n";
    file.write_all(out.as_bytes()).await.unwrap();
}

pub fn log(s: &str) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("log.txt")
        .unwrap();
    let out: String = s.to_string();
    file.write_all(out.as_bytes()).unwrap();
}
