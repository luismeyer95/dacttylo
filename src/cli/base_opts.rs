use super::{HostOptions, JoinOptions, PracticeOptions};

pub trait BaseOpts {
    fn get_username(&self) -> Option<&str>;
}

impl BaseOpts for HostOptions {
    fn get_username(&self) -> Option<&str> {
        Some(&self.username)
    }
}

impl BaseOpts for JoinOptions {
    fn get_username(&self) -> Option<&str> {
        Some(&self.username)
    }
}

impl BaseOpts for PracticeOptions {
    fn get_username(&self) -> Option<&str> {
        self.username.as_deref()
    }
}
