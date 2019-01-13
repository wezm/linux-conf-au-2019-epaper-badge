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
use profont::{ProFont14Point, ProFont24Point, ProFont9Point};

// HTTP Server
use futures::Future;
use hyper::service::service_fn;
use hyper::Server;
use std::net::SocketAddr;

// System info
use nix::sys::utsname::uname;
use rs_release::get_os_release;
use systemstat::{IpAddr, Ipv4Addr, Platform, System};

use std::process::Command;
// use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;

use crate::app::State;
use crate::system::Uptime;

const ROWS: u16 = 212;
const COLS: u8 = 104;
const LISTEN_ADDR: &str = "0.0.0.0";

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

    /// Don't loop, just run once and exit
    #[structopt(short = "o", long)]
    oneshot: bool,
}

fn main() -> Result<(), std::io::Error> {
    let options = Options::from_args();

    let system = System::new();
    let wlan0_address = get_interface_ip(&system, &options.interface);
    let os_name = get_os_release()
        .ok()
        .and_then(|mut hash| hash.remove("NAME"))
        .unwrap_or_else(|| "Unknown".to_string());

    let state = Arc::new(RwLock::new(State {
        hi_count: 0,
        ip: wlan0_address,
        os_name: os_name,
        uname: uname(),
        memory: system.memory().ok(),
        uptime: system
            .uptime()
            .ok()
            .map(|uptime| Uptime::new(uptime.as_secs()))
            .unwrap_or_default(),
    }));

    if !options.nodisplay {
        let mut delay = Delay {};

        let display = hardware::display(COLS, ROWS)?;

        let mut black_buffer = [0u8; ROWS as usize * COLS as usize / 8];
        let mut red_buffer = [0u8; ROWS as usize * COLS as usize / 8];
        let mut display = GraphicDisplay::new(display, &mut black_buffer, &mut red_buffer);

        // TODO: Schedule this as a tokio timer
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
                    .translate(Coord::new(1, 20))
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
            display.draw(
                ProFont14Point::render_str("curl")
                    .with_stroke(Some(Color::Black))
                    .with_fill(Some(Color::White))
                    .translate(Coord::new(1, 45))
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

            if options.oneshot {
                break;
            }

            sleep(one_minute);
        }
    }

    if !options.noserver {
        // TODO: Implement shutdown?
        let new_service = move || {
            // FIXME: This double clone doesn't seem right... but works
            let state = state.clone();

            service_fn(move |req| webserver::handle_request(state.clone(), req))
        };

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
