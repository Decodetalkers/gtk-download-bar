use std::process;

use anyhow::Result;
use gtk::prelude::*;
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
    let start = gtkdownloadbar::DownloadProgressBar::new(
        "https://d.store.deepinos.org.cn//store/chat/chaoxin/chaoxin_1.8.3_amd64.deb".to_string(),
    )?;
    let progress_bar = start.progress_bar();
    window.add(&progress_bar);
    window.show_all();

    Ok(())
}
