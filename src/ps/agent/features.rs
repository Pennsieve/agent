//! Tests if specific features are enabled in the Pennsieve agent.

use std::env;

#[allow(dead_code)]
/// Feature: show file upload progress bar
pub fn show_progress_bar() -> bool {
    if let Ok(text) = env::var("PS_PROGRESS_BAR") {
        if text == "0" || text == "false" || text == "no" {
            return false;
        }
    }
    true
}
