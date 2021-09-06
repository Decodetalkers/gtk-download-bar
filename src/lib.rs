mod config;
mod core;
mod download;
mod utils;
use download::*;
use gtk::prelude::*;
use gtk::{Box, Button, ProgressBar};
use std::cell::RefCell;
use std::thread;
pub struct DownloadProgressBar {
    status: RefCell<bool>,
}
impl DownloadProgressBar {
    pub fn new() -> Self {
        Self {
            status: RefCell::new(true),
        }
    }
    pub fn progress_bar(self, url: String) -> Box {
        let progress_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let download_bar = ProgressBar::new();
        let download_button = Button::with_label("start");
        progress_bar.pack_start(&download_bar, true, true, 0);
        progress_bar.pack_start(&download_button, false, false, 0);
        download_button.connect_clicked(move |button|{
            let status = *self.status.borrow();
            if status {
                let url = utils::parse_url(&url).unwrap();
                let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
                thread::spawn(move || match url.scheme() {
                    "ftp" => ftp_download(tx, url, false, None),
                    "http" | "https" => http_download(tx, url),
                    _ => utils::gen_error(format!("unsupported url scheme '{}'", url.scheme())),
                });
                button.hide();
                rx.attach(None, glib::clone!(@weak download_bar, @weak button=> @default-return glib::Continue(false),move |value| match value{
                    Some(length)=>{
                        download_bar.set_fraction(length);
                        glib::Continue(true)
                    },
                    None => {
                        println!("finish");
                        button.show();
                        button.set_label("Finish");
                        glib::Continue(false)
                    }
                }));
                *self.status.borrow_mut() = false;
            } else {
                println!("Start Install");
            }


        });
        progress_bar
    }
}
