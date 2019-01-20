mod app;
mod hardware;
mod system;
mod webserver;

use structopt::StructOpt;

use linux_embedded_hal::Delay;

use ssd1675::{Color, GraphicDisplay};

// Graphics
use embedded_graphics::coord::Coord;
use embedded_graphics::prelude::*;
use embedded_graphics::Drawing;

// Font
use profont::{ProFont12Point, ProFont14Point, ProFont24Point};

// HTTP Server
use futures::Future;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::net::SocketAddr;

// System info
use nix::sys::utsname::uname;
use rs_release::get_os_release;
use systemstat::{IpAddr, Ipv4Addr, Platform, System};

use std::alloc;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use crate::app::State;
use crate::system::Uptime;

#[global_allocator]
static GLOBAL: alloc::System = alloc::System;

const ROWS: u16 = 212;
const COLS: u8 = 104;
const LISTEN_ADDR: &str = "0.0.0.0";

#[derive(StructOpt, Debug, Clone)]
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

    /// Don't loop, just run once and exit
    #[structopt(short = "o", long)]
    oneshot: bool,
}

#[derive(Debug, PartialEq)]
struct DisplayState {
    hi_count: usize,
    ip: Option<Ipv4Addr>,
}

fn main() -> Result<(), std::io::Error> {
    let options = Options::from_args();

    let system = System::new();
    let wlan0_address = get_interface_ip(&system, &options.interface);
    let os_name = get_os_release()
        .ok()
        .and_then(|mut hash| hash.remove("NAME"))
        .unwrap_or_else(|| "Unknown".to_string());

    let save_path = Path::new("hi_count.txt");
    let hello_max_age = Duration::from_secs(60 * 60);

    let state = Arc::new(RwLock::new(State::load(
        &save_path,
        wlan0_address,
        os_name,
        uname(),
        system.memory().ok(),
        system
            .uptime()
            .ok()
            .map(|uptime| Uptime::new(uptime.as_secs()))
            .unwrap_or_default(),
        hello_max_age,
    )?));

    let display_thread = if !options.nodisplay {
        let options = options.clone();
        let state = state.clone();

        Some(thread::spawn(move || {
            let mut delay = Delay {};

            let display = hardware::display(COLS, ROWS).expect("unable to create display");

            let mut black_buffer = [0u8; ROWS as usize * COLS as usize / 8];
            let mut red_buffer = [0u8; ROWS as usize * COLS as usize / 8];
            let mut display = GraphicDisplay::new(display, &mut black_buffer, &mut red_buffer);
            let mut old_display_state = DisplayState {
                hi_count: 1,
                ip: get_interface_ip(&system, &options.interface),
            };
            let update_delay = Duration::from_secs(15);

            loop {
                // TODO: Extract this to an update display function
                let display_state = {
                    let state = state.read().expect("poisioned");
                    let new_display_state = DisplayState {
                        hi_count: state.hi_count(),
                        ip: get_interface_ip(&system, &options.interface),
                    };

                    if new_display_state.hi_count != old_display_state.hi_count {
                        if let Err(err) = state.save_hi_count(&save_path) {
                            println!("unable to save hi count: {:?}", err);
                        }
                    }

                    new_display_state
                };

                if display_state != old_display_state {
                    match display.reset(&mut delay) {
                        Ok(()) => println!("Reset and initialised"),
                        Err(err) => println!("Error resetting display: {:?}", err),
                    }

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
                            .translate(Coord::new(1, 22))
                            .into_iter(),
                    );

                    let hi = display_state.hi_count.to_string();
                    display.draw(
                        ProFont24Point::render_str(&hi)
                            .with_stroke(Some(Color::Black))
                            .with_fill(Some(Color::White))
                            .translate(Coord::new(1, 42))
                            .into_iter(),
                    );
                    let unit = if display_state.hi_count != 1 {
                        "people have"
                    } else {
                        "person has"
                    };
                    display.draw(
                        ProFont12Point::render_str(unit)
                            .with_stroke(Some(Color::Black))
                            .with_fill(Some(Color::White))
                            .translate(Coord::new(hi.len() as i32 * 17 + 4, 43))
                            .into_iter(),
                    );
                    display.draw(
                        ProFont12Point::render_str("said hello")
                            .with_stroke(Some(Color::Black))
                            .with_fill(Some(Color::White))
                            .translate(Coord::new(hi.len() as i32 * 17 + 4, 57))
                            .into_iter(),
                    );

                    let ip = display_state
                        .ip
                        .map(|ip| ip.to_string())
                        .unwrap_or_else(|| "?.?.?.?".to_string());
                    // http://192.168.100.100/ = 23 chars 230px
                    // http://10.0.0.18/ = 17 chars 170px
                    let url = format!("http://{}/", ip);

                    // If URL is longer than will fit on the screen use a smaller font
                    // 14pt is 10px wide
                    // 12pt is 8px wide
                    if url.len() * 10 > ROWS as usize {
                        display.draw(
                            ProFont12Point::render_str(&url)
                                .with_stroke(Some(Color::Black))
                                .with_fill(Some(Color::White))
                                .translate(Coord::new(1, 90))
                                .into_iter(),
                        );
                    } else {
                        display.draw(
                            ProFont14Point::render_str(&url)
                                .with_stroke(Some(Color::Black))
                                .with_fill(Some(Color::White))
                                .translate(Coord::new(1, 88))
                                .into_iter(),
                        );
                    }
                    display.draw(
                        ProFont14Point::render_str("Say hi at:")
                            .with_stroke(Some(Color::Black))
                            .with_fill(Some(Color::White))
                            .translate(Coord::new(1, 73))
                            .into_iter(),
                    );

                    match display.update(&mut delay) {
                        Ok(()) => println!("Update..."),
                        Err(err) => println!("error updating display: {:?}", err),
                    }

                    match display.deep_sleep() {
                        Ok(()) => println!("Finished - going to sleep"),
                        Err(err) => println!("Error going to sleep: {:?}", err),
                    }
                } else {
                    println!("No change, skip display update");
                }

                old_display_state = display_state;

                if options.oneshot {
                    break;
                }

                thread::sleep(update_delay);
            }
        }))
    } else {
        None
    };

    if !options.noserver {
        // TODO: Implement shutdown?
        let new_service = make_service_fn(move |socket: &AddrStream| {
            let remote_addr = socket.remote_addr();

            // This double clone doesn't seem right... but works
            let state = state.clone();

            service_fn(move |req| webserver::handle_request(state.clone(), remote_addr, req))
        });

        let addr = SocketAddr::from((LISTEN_ADDR.parse::<Ipv4Addr>().unwrap(), options.port));
        let server = Server::bind(&addr)
            .serve(new_service)
            .map_err(|e| eprintln!("server error: {}", e));

        println!(
            "Starting server on http:://{}",
            wlan0_address
                .map(|ip| format!("{}:{}", ip, options.port))
                .unwrap_or_else(|| format!("{}:{}", LISTEN_ADDR, options.port))
        );

        hyper::rt::run(server);
    }

    match (options.oneshot, display_thread) {
        (true, Some(thread)) => {
            let _ = thread.join();
        }
        _ => (),
    }

    Ok(())
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
