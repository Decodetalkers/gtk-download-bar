use std::process;

use anyhow::{format_err, Result};
use clap::{clap_app, crate_version};
use duma::download::{ftp_download, http_download};
use duma::utils;
use gtk::prelude::*;
use std::thread;
fn main() {    
    let application = gtk::Application::new(Some("com.ssss"),gio::ApplicationFlags::HANDLES_OPEN);
    application.connect_open(|window,_,_|{
        match run(window) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        }
    });
    application.connect_activate(|window|{
        match run(window) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("error: {}", e);
                process::exit(1);
            }
        }
    });
    application.run();
}

fn run(application: &gtk::Application) -> Result<()> {
    let window = gtk::ApplicationWindow::new(application);
    window.set_title("Accessibility");
    window.set_position(gtk::WindowPosition::Center);
    let args = clap_app!(Duma =>
    (version: crate_version!())
    (author: "Matt Gathu <mattgathu@gmail.com>")
    (about: "A minimal file downloader")
    (@arg quiet: -q --quiet "quiet (no output)")
    (@arg continue: -c --continue "resume getting a partially-downloaded file")
    (@arg singlethread: -s --singlethread "download using only a single thread")
    (@arg headers: -H --headers "prints the headers sent by the HTTP server")
    (@arg FILE: -O --output +takes_value "write documents to FILE")
    (@arg AGENT: -U --useragent +takes_value "identify as AGENT instead of Duma/VERSION")
    (@arg SECONDS: -T --timeout +takes_value "set all timeout values to SECONDS")
    (@arg NUM_CONNECTIONS: -n --num_connections +takes_value "maximum number of concurrent connections (default is 8)")
    (@arg URL: +required +takes_value "url to download")
    )
    .get_matches_safe().unwrap_or_else(|e| e.exit());
    let args2 =args.clone(); 
    let url = utils::parse_url(
        args.value_of("URL")
            .ok_or_else(|| format_err!("missing URL argument"))?,
    )?;
    let quiet_mode = args.is_present("quiet");
    let progress_bar = gtk::ProgressBar::new();
    let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
    thread::spawn(move ||{
        match url.scheme() {
            "ftp" => {
                let file_name = args2.value_of("FILE");
                ftp_download(tx,url, quiet_mode, file_name)
            },
            "http" | "https" => http_download(tx,url, &args2),
            _ => utils::gen_error(format!("unsupported url scheme '{}'", url.scheme())),
        }
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
