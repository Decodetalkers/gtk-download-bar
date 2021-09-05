use std::process;

use anyhow::Result;
use duma::download::{ftp_download, http_download};
use duma::utils;
use gtk::prelude::*;
use std::thread;
fn main() {
    let application = gtk::Application::new(Some("com.ssss"), gio::ApplicationFlags::HANDLES_OPEN);
    application.connect_open(|window, _, _| match run(window) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    });
    application.connect_activate(|window| match run(window) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    });
    application.run();
}

fn run(application: &gtk::Application) -> Result<()> {
    let window = gtk::ApplicationWindow::new(application);
    window.set_title("Accessibility");
    window.set_position(gtk::WindowPosition::Center);
    let url = utils::parse_url(
        "https://d.store.deepinos.org.cn//store/chat/chaoxin/chaoxin_1.8.3_amd64.deb",
    )?;
    let progress_bar = gtk::ProgressBar::new();
    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    thread::spawn(move || match url.scheme() {
        "ftp" => ftp_download(tx, url, false, None),
        "http" | "https" => http_download(tx, url),
        _ => utils::gen_error(format!("unsupported url scheme '{}'", url.scheme())),
    });

    rx.attach(None, glib::clone!(@weak progress_bar=> @default-return glib::Continue(false),move |value| match value{
        Some(length)=>{
            progress_bar.set_fraction(length);
            glib::Continue(true)
        },
        None => {
            println!("finish");
            glib::Continue(false)
        }
    }));
    window.add(&progress_bar);
    window.show_all();

    Ok(())
}
