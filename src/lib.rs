use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod server;
pub mod sql;
pub mod twitter;
pub mod worker;

pub fn current_time_duration() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get current UNIX time.")
}
