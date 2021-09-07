use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{format_err, Result};
use console::style;
use indicatif::HumanBytes;
use reqwest::blocking::Client;
use reqwest::header::{self, HeaderMap, HeaderValue};

use url::Url;

use crate::config::*;
use crate::core::{Config, EventsHandler, FtpDownload, HttpDownload};
use crate::utils::{decode_percent_encoded_data, get_file_handle};
fn create_storage_before() {
    fs::create_dir_all(DIR).unwrap();
}
pub fn request_headers_from_server(url: &Url, timeout: u64, ua: &str) -> Result<HeaderMap> {
    let resp = Client::new()
        .get(url.as_ref())
        .timeout(Duration::from_secs(timeout))
        .header(header::USER_AGENT, HeaderValue::from_str(ua)?)
        .header(header::ACCEPT, HeaderValue::from_str("*/*")?)
        .send()?;
    Ok(resp.headers().clone())
}

pub fn gen_filename(url: &Url, fname: Option<&str>, headers: Option<&HeaderMap>) -> String {
    let content_disposition = headers
        .and_then(|hdrs| hdrs.get(header::CONTENT_DISPOSITION))
        .and_then(|val| {
            let val = val.to_str().unwrap_or("");
            if val.contains("filename=") {
                Some(val)
            } else {
                None
            }
        })
        .and_then(|val| {
            let x = val
                .rsplit(';')
                .next()
                .unwrap_or("")
                .rsplit('=')
                .next()
                .unwrap_or("")
                .trim_start_matches('"')
                .trim_end_matches('"');
            if !x.is_empty() {
                Some(x.to_string())
            } else {
                None
            }
        });
    match fname {
        Some(name) => name.to_owned(),
        None => match content_disposition {
            Some(val) => val,
            None => {
                let name = &url.path().split('/').last().unwrap_or("");
                if !name.is_empty() {
                    match decode_percent_encoded_data(name) {
                        Ok(val) => val,
                        _ => name.to_string(),
                    }
                } else {
                    "index.html".to_owned()
                }
            }
        },
    }
}

// 保存了临时文件
fn calc_bytes_on_disk(fname: &str) -> Result<Option<u64>> {
    // use state file if present
    let st_fname = format!("{}{}.st", DIR, fname);
    if Path::new(&st_fname).exists() {
        let input = fs::File::open(st_fname)?;
        let buf = BufReader::new(input);
        let mut byte_count: u64 = 0;
        for line in buf.lines() {
            let num_of_bytes = line?
                .split(':')
                .next()
                .ok_or_else(|| format_err!("failed to split state file line"))?
                .parse::<u64>()?;
            byte_count += num_of_bytes;
        }
        return Ok(Some(byte_count));
    }
    match fs::metadata(fname) {
        Ok(metadata) => Ok(Some(metadata.len())),
        _ => Ok(None),
    }
}

fn prep_headers(fname: &str, resume: bool, user_agent: &str) -> Result<HeaderMap> {
    let bytes_on_disk = calc_bytes_on_disk(fname)?;
    let mut headers = HeaderMap::new();
    if let Some(bcount) = bytes_on_disk {
        if resume {
            let byte_range = format!("bytes={}-", bcount);
            headers.insert(header::RANGE, byte_range.parse()?);
        }
    }

    headers.insert(header::USER_AGENT, user_agent.parse()?);

    Ok(headers)
}

pub fn ftp_download(
    prog_bar: glib::Sender<Option<(f64,String)>>,
    url: Url,
    quiet_mode: bool,
    filename: Option<&str>,
) -> Result<()> {
    create_storage_before();
    let fname = gen_filename(&url, filename, None);

    let mut client = FtpDownload::new(url);
    let events_handler = DefaultEventsHandler::new(prog_bar, &fname, false, false, quiet_mode)?;
    client.events_hook(events_handler).download()?;
    Ok(())
}

// http 下载入口
pub fn http_download(prog_bar: glib::Sender<Option<(f64,String)>>, url: Url) -> Result<()> {
    create_storage_before();
    let user_agent = "".to_string();
    let timeout = 30u64;
    let num_workers = 8usize;
    let headers = request_headers_from_server(&url, timeout, &user_agent)?;
    let fname = gen_filename(&url, None, Some(&headers));
    // early exit if headers flag is present

    // 这边跳转了个新的function,处理内容未知
    let headers = prep_headers(&fname, false, &user_agent)?;

    // 返回一个临时文件是否存在的bool
    let chunk_size = 512_000u64;

    let chunk_offsets = None;

    let bytes_on_disk = None;

    // 生成了一个conifg,来自core
    let conf = Config {
        user_agent,
        resume: false,
        headers,
        file: fname.clone(),
        timeout,
        concurrent: true,
        max_retries: 100,
        num_workers,
        bytes_on_disk,
        chunk_offsets,
        chunk_size,
    };

    let mut client = HttpDownload::new(url, conf);

    let events_handler = DefaultEventsHandler::new(prog_bar, &fname, false, true, false)?;
    client.events_hook(events_handler).download()?;
    Ok(())
}

pub struct DefaultEventsHandler {
    // construct the progessbar
    prog_bar: glib::Sender<Option<(f64,String)>>,
    progress: f64,
    length: u64,
    bytes_on_disk: Option<u64>,
    fname: String,
    file: BufWriter<fs::File>,
    st_file: Option<BufWriter<fs::File>>,
    server_supports_resume: bool,
    quiet_mode: bool,
}

// handle the event
impl DefaultEventsHandler {
    pub fn new(
        prog_bar: glib::Sender<Option<(f64,String)>>,
        fname: &str,
        resume: bool,
        concurrent: bool,
        quiet_mode: bool,
    ) -> Result<DefaultEventsHandler> {
        let st_file = if concurrent {
            Some(BufWriter::new(get_file_handle(
                &format!("{}.st", fname),
                resume,
                true,
            )?))
        } else {
            None
        };
        Ok(DefaultEventsHandler {
            prog_bar,
            length: 0,
            progress: 0.0,
            bytes_on_disk: calc_bytes_on_disk(fname)?,
            fname: fname.to_owned(),
            file: BufWriter::new(get_file_handle(fname, resume, !concurrent)?),
            st_file,
            server_supports_resume: false,
            quiet_mode,
        })
    }

    fn create_prog_bar(&mut self, length: Option<u64>) {
        let byte_count = if self.server_supports_resume {
            self.bytes_on_disk
        } else {
            None
        };
        if let Some(len) = length {
            let exact = style(len).green();
            let human_readable = style(format!("{}", HumanBytes(len))).red();

            println!("Length: {} ({})", exact, human_readable);

            self.length = len;
        } else {
            println!("Length: {}", style("unknown").red());
        }

        //let prog_bar = create_progress_bar(&self.fname, length);
        if let Some(count) = byte_count {
            let timeline = format!("receive {}/ total{}",HumanBytes(count),HumanBytes(self.length));
            self.progress += count as f64 / (self.length as f64);
            self.prog_bar
                .send(Some((self.progress,timeline)))
                .expect("cannot send");
        }
        //self.prog_bar = Some(prog_bar);
    }
}

impl EventsHandler for DefaultEventsHandler {
    fn on_headers(&mut self, headers: HeaderMap) {
        if self.quiet_mode {
            return;
        }
        let ct_type = if let Some(val) = headers.get(header::CONTENT_TYPE) {
            val.to_str().unwrap_or("")
        } else {
            ""
        };
        println!("Type: {}", style(ct_type).green());

        println!("Saving to: {}", style(&self.fname).green());
        if let Some(val) = headers.get(header::CONTENT_LENGTH) {
            self.create_prog_bar(val.to_str().unwrap_or("").parse::<u64>().ok());
        } else {
            println!(
                "{}",
                style("Got no content-length. Progress bar skipped.").red()
            );
        }
    }

    fn on_ftp_content_length(&mut self, ct_len: Option<u64>) {
        if !self.quiet_mode {
            self.create_prog_bar(ct_len);
        }
    }

    fn on_server_supports_resume(&mut self) {
        self.server_supports_resume = true;
    }

    fn on_content(&mut self, content: &[u8]) -> Result<()> {
        let byte_count = content.len() as u64;
        self.file.write_all(content)?;
        let timeline = format!("receive {}/ total{}",HumanBytes(byte_count),HumanBytes(self.length));
        self.progress += (byte_count as f64) / (self.length as f64);
        self.prog_bar.send(Some((self.progress,timeline)))?;
        Ok(())
    }
    // 这里更新thread的内容
    //
    //
    // 如果要移植改这里
    fn on_concurrent_content(&mut self, content: (u64, u64, &[u8])) -> Result<()> {
        let (byte_count, offset, buf) = content;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(buf)?;
        self.file.flush()?;
        let timeline = format!("receive {}/ total{}",HumanBytes(byte_count),HumanBytes(self.length));
        self.progress += (byte_count as f64) / (self.length as f64);
        self.prog_bar.send(Some((self.progress,timeline)))?;
        if let Some(ref mut file) = self.st_file {
            writeln!(file, "{}:{}", byte_count, offset)?;
            file.flush()?;
        }
        Ok(())
    }

    fn on_resume_download(&mut self, bytes_on_disk: u64) {
        self.bytes_on_disk = Some(bytes_on_disk);
    }

    fn on_finish(&mut self) {
        let timeline = format!("Finished/ total{}",HumanBytes(self.length));
        self.prog_bar.send(Some((1.0,timeline))).expect("error");
        self.prog_bar.send(None).expect("error");
        if fs::remove_file(&format!("{}{}.st", DIR, self.fname)).is_ok() {};
    }

    fn on_max_retries(&mut self) {
        if !self.quiet_mode {
            eprintln!("{}", style("max retries exceeded. Quitting!").red());
        }
        if self.file.flush().is_ok() {}
        if let Some(ref mut file) = self.st_file {
            if file.flush().is_ok() {};
        }
        ::std::process::exit(0);
    }

    fn on_failure_status(&self, status: i32) {
        if self.quiet_mode {
            return;
        }
        if status == 416 {
            println!(
                "{}",
                &style("\nThe file is already fully retrieved; nothing to do.\n").red()
            );
        }
    }
}
