pub mod config;
pub mod core;
pub mod download;
pub mod utils;
use gtk::prelude::*;
use download::*;
use std::thread;
use gtk::{Box, Button, ProgressBar, prelude::{BoxExt, ButtonExt}};
pub struct DownloadProgressBar;
impl DownloadProgressBar {
    pub fn new(url: String) -> Box {
        let progress_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let download_bar = ProgressBar::new();
        let download_button = Button::with_label("start");
        progress_bar.pack_start(&download_bar, true, true, 0);
        progress_bar.pack_start(&download_button, false, false, 0);
        download_button.connect_clicked(move |button|{
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


        });
        progress_bar
    }
}
