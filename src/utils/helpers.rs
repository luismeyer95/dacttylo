pub fn is_valid_file(val: &str) -> Result<(), String> {
    if std::path::Path::new(val).exists() {
        Ok(())
    } else {
        Err(format!("file `{}` does not exist.", val))
    }
}
