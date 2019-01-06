use structopt::StructOpt;

use linux_embedded_hal::Delay;

use ssd1675::{Color, GraphicDisplay};

// Graphics
use embedded_graphics::coord::Coord;
use embedded_graphics::prelude::*;
use embedded_graphics::Drawing;

// Font
use profont::{ProFont14Point, ProFont18Point, ProFont24Point, ProFont9Point};

// HTTP Server
use askama::Template;
use tiny_http;

// System info
use nix::sys::utsname::{uname, UtsName};
use rs_release::get_os_release;
use systemstat::{ByteSize, IpAddr, Ipv4Addr, Memory, Platform, System};

use std::fmt;
use std::process::Command;
// use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use lca2019::hardware;

const ROWS: u16 = 212;
const COLS: u8 = 104;
const THREADS: usize = 2;
const LISTEN_ADDR: &str = "0.0.0.0";
const ONE_DAY: u64 = 24 * 60 * 60;
const ONE_HOUR: u64 = 60 * 60;

const FERRIS: &str = include_str!("ferris.txt");

#[derive(StructOpt, Debug)]
#[structopt(name = "lca2019", about = "linux.conf.au 2019 conference badge.")]
struct Options {
    /// HTTP server port
    #[structopt(short, long, default_value = "80")]
    port: u16,

    /// Interface whose IP is shown on the display
    #[structopt(short, long, default_value = "wlan0")]
    interface: String,

    /// Don't try to update the ePaper display
    #[structopt(short, long)]
    nodisplay: bool,

    /// Disable the HTTP server
    #[structopt(short = "s", long)]
    noserver: bool,
}

#[derive(Template)]
#[template(path = "hi.txt")]
struct HelloTemplate<'a> {
    ip: &'a str,
    os_name: &'a str,
    uname: &'a UtsName,
    memory: Option<Memory>,
    uptime: Uptime,
}

#[derive(Debug, Copy, Clone, Default)]
struct Uptime(u64);

impl Uptime {
    pub fn new(seconds: u64) -> Self {
        Uptime(seconds)
    }
}

impl fmt::Display for Uptime {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let days = self.0 / ONE_DAY;
        let total_hours = self.0 % ONE_DAY;
        let hours = total_hours / ONE_HOUR;
        let minutes = total_hours % ONE_HOUR / 60;

        let mut fragments = vec![];
        if days > 0 {
            fragments.push(format!("{}d", days));
        }
        if hours > 0 {
            fragments.push(format!("{}h", hours));
        }
        if minutes > 0 {
            fragments.push(format!("{}m", minutes));
        }
        if days == 0 && hours == 0 && minutes == 0 {
            fragments.push(format!("{} secs", self.0));
        }

        f.write_str(&fragments.join(" "))
    }
}

fn main() -> Result<(), std::io::Error> {
    let options = Options::from_args();

    let system = Arc::new(System::new());
    let wlan0_address = get_interface_ip(&system, &options.interface);
    let os_name = Arc::new(
        get_os_release()
            .ok()
            .and_then(|mut hash| hash.remove("NAME"))
            .unwrap_or_else(|| "Unknown".to_string()),
    );
    let mut threads = Vec::new();
    if !options.noserver {
        let server = Arc::new(tiny_http::Server::http((LISTEN_ADDR, options.port)).unwrap());
        println!(
            "Now listening on http:://{}",
            wlan0_address
                .map(|ip| format!("{}:{}", ip, options.port))
                .unwrap_or_else(|| format!("{}:{}", LISTEN_ADDR, options.port))
        );

        // let (tx, rx) = mpsc::channel();

        for _ in 0..THREADS {
            let server = server.clone();
            let system = system.clone();
            let os_name = os_name.clone();
            let utsname = uname();

            threads.push(thread::spawn(move || {
                for req in server.incoming_requests() {
                    let ip_string = wlan0_address
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|| "?.?.?.?".to_string());
                    let uptime = system
                        .uptime()
                        .ok()
                        .map(|uptime| Uptime::new(uptime.as_secs()))
                        .unwrap_or_default();
                    let template = HelloTemplate {
                        ip: &ip_string,
                        memory: system.memory().ok(),
                        uptime,
                        os_name: &os_name,
                        uname: &utsname,
                    };

                    let response_data = template
                        .render()
                        .ok()
                        .unwrap_or_else(|| "Internal Server Error".to_string());

                    // GET /
                    let response = tiny_http::Response::from_string(response_data);
                    let _ = req.respond(response);

                    // POST /hi

                    // Else 404
                }
            }));
        }
    }

    if !options.nodisplay {
        let mut delay = Delay {};

        let display = hardware::display(COLS, ROWS)?;

        let mut black_buffer = [0u8; ROWS as usize * COLS as usize / 8];
        let mut red_buffer = [0u8; ROWS as usize * COLS as usize / 8];
        let mut display = GraphicDisplay::new(display, &mut black_buffer, &mut red_buffer);

        loop {
            display.reset(&mut delay).expect("error resetting display");
            println!("Reset and initialised");
            let one_minute = Duration::from_secs(60);

            display.clear(Color::White);
            println!("Clear");

            display.draw(
                ProFont24Point::render_str("Wesley Moore")
                    .with_stroke(Some(Color::Red))
                    .with_fill(Some(Color::White))
                    .translate(Coord::new(1, -4))
                    .into_iter(),
            );

            display.draw(
                ProFont14Point::render_str("wezm.net")
                    .with_stroke(Some(Color::Black))
                    .with_fill(Some(Color::White))
                    .translate(Coord::new(1, 24))
                    .into_iter(),
            );

            let ip = get_interface_ip(&system, &options.interface)
                .map(|ip| ip.to_string())
                .unwrap_or_else(|| "?.?.?.?".to_string());
            display.draw(
                ProFont14Point::render_str(&format!("http://{}/", ip))
                    .with_stroke(Some(Color::Black))
                    .with_fill(Some(Color::White))
                    .translate(Coord::new(1, 58))
                    .into_iter(),
            );

            if let Some(uname) = read_uname() {
                display.draw(
                    ProFont9Point::render_str(uname.trim())
                        .with_stroke(Some(Color::Black))
                        .with_fill(Some(Color::White))
                        .translate(Coord::new(1, 93))
                        .into_iter(),
                );
            }

            display.update(&mut delay).expect("error updating display");
            println!("Update...");

            println!("Finished - going to sleep");
            display.deep_sleep()?;

            sleep(one_minute);
        }
    }

    // TODO: Implement shutdown?
    for thread in threads {
        thread.join().unwrap();
    }

    Ok(())
}

fn read_uname() -> Option<String> {
    Command::new("uname")
        .arg("-smr")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
}

fn get_interface_ip(system: &System, interface: &str) -> Option<Ipv4Addr> {
    if let Some(network) = system
        .networks()
        .ok()
        .and_then(|mut info| info.remove(interface))
    {
        for addr in network.addrs {
            match addr.addr {
                IpAddr::V4(addr) => return Some(addr),
                _ => (),
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uptime_days() {
        let uptime = Uptime::new(946560);
        assert_eq!(uptime.to_string(), String::from("10d 22h 56m"));
    }

    #[test]
    fn test_uptime_hours() {
        let uptime = Uptime::new(12345);
        assert_eq!(uptime.to_string(), String::from("3h 25m"));
    }

    #[test]
    fn test_uptime_minutes() {
        let uptime = Uptime::new(62);
        assert_eq!(uptime.to_string(), String::from("1m"));
    }

    #[test]
    fn test_uptime_seconds() {
        let uptime = Uptime::new(42);
        assert_eq!(uptime.to_string(), String::from("42 secs"));
    }
}
