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
use tiny_http;
use askama::Template;

// System info
use systemstat::{IpAddr, Ipv4Addr, Platform, System};

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
}

fn main() -> Result<(), std::io::Error> {
    let options = Options::from_args();

    let system = System::new();
    let wlan0_address = get_interface_ip(&system, &options.interface);

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

            threads.push(thread::spawn(move || {
                for req in server.incoming_requests() {
                    let ip_string = wlan0_address.map(|ip| ip.to_string()).unwrap_or_else(|| "?.?.?.?".to_string());
                    let template = HelloTemplate { ip: &ip_string };

                    let response_data = template.render().ok().unwrap_or_else(|| "Internal Server Error".to_string());

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
