mod config;
mod core;
mod download;
mod utils;
use anyhow::Result;
use config::DIR;
use download::*;
use gtk::prelude::*;
use gtk::{Box, Button, ProgressBar};
use std::{cell::RefCell, path::Path, thread};
pub struct DownloadProgressBar {
    status: RefCell<bool>,
    url: String,
    fname: String,
}
impl DownloadProgressBar {
    pub fn new(url: String) -> Result<Self> {
        let urll = utils::parse_url(&url).unwrap();
        let headers = request_headers_from_server(&urll, 30u64, "")?;
        let fname = gen_filename(&urll, None, Some(&headers));
        Ok(Self {
            status: RefCell::new(true),
            url,
            fname,
        })
    }
    pub fn progress_bar(self) -> Box {
        let url = self.url.clone();
        let progress_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let download_bar = ProgressBar::new();
        let download_button = Button::with_label("start");
        progress_bar.pack_start(&download_bar, true, true, 0);
        progress_bar.pack_start(&download_button, false, false, 0);
        download_button.connect_clicked(move |button|{
            let status = *self.status.borrow();
            if status {
                if !Path::new(&format!("{}{}",DIR,self.fname)).exists(){
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
                } else {
                    download_bar.set_fraction(1f64);
                    button.set_label("Finish");
                }
                *self.status.borrow_mut() = false;
            } else  if Path::new(&format!("{}{}",DIR,self.fname)).exists(){
                println!("Start Install");
            }else{
                *self.status.borrow_mut() = true;
                button.set_label("start");
            }
        });
        progress_bar
    }
}
