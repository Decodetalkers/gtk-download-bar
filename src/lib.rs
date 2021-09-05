pub mod config;
pub mod core;
pub mod download;
pub mod utils;
use gtk::{Box, Button, ProgressBar};
struct DownloadProgressBar;
impl DownloadProgressBar {
    fn new() -> Box {
        let progress_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        progress_bar
    }
}
