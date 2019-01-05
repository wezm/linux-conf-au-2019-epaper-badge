extern crate linux_embedded_hal;
use linux_embedded_hal::Delay;

extern crate ssd1675;
use ssd1675::{Color, GraphicDisplay};

// Graphics
extern crate embedded_graphics;
use embedded_graphics::coord::Coord;
use embedded_graphics::prelude::*;
use embedded_graphics::Drawing;

// Font
extern crate profont;
use profont::{ProFont12Point, ProFont14Point, ProFont24Point, ProFont9Point};

use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};

use lca2019::hardware;

// Activate SPI, GPIO in raspi-config needs to be run with sudo because of some sysfs_gpio
// permission problems and follow-up timing problems
// see https://github.com/rust-embedded/rust-sysfs-gpio/issues/5 and follow-up issues

const ROWS: u16 = 212;
const COLS: u8 = 104;

fn main() -> Result<(), std::io::Error> {
    let mut delay = Delay {};

    let display = hardware::display(COLS, ROWS)?;

    let mut black_buffer = [0u8; ROWS as usize * COLS as usize / 8];
    let mut red_buffer = [0u8; ROWS as usize * COLS as usize / 8];
    let mut display = GraphicDisplay::new(display, &mut black_buffer, &mut red_buffer);

    // Main loop. Displays CPU temperature, uname, and uptime every minute with a red Raspberry Pi
    // header.
    loop {
        display.reset(&mut delay).expect("error resetting display");
        println!("Reset and initialised");
        let one_minute = Duration::from_secs(60);

        display.clear(Color::White);
        println!("Clear");

        display.draw(
            ProFont24Point::render_str("Raspberry Pi")
                .with_stroke(Some(Color::Red))
                .with_fill(Some(Color::White))
                .translate(Coord::new(1, -4))
                .into_iter(),
        );

        if let Ok(cpu_temp) = read_cpu_temp() {
            display.draw(
                ProFont14Point::render_str("CPU Temp:")
                    .with_stroke(Some(Color::Black))
                    .with_fill(Some(Color::White))
                    .translate(Coord::new(1, 30))
                    .into_iter(),
            );
            display.draw(
                ProFont12Point::render_str(&format!("{:.1}°C", cpu_temp))
                    .with_stroke(Some(Color::Black))
                    .with_fill(Some(Color::White))
                    .translate(Coord::new(95, 34))
                    .into_iter(),
            );
        }

        if let Some(uptime) = read_uptime() {
            display.draw(
                ProFont9Point::render_str(uptime.trim())
                    .with_stroke(Some(Color::Black))
                    .with_fill(Some(Color::White))
                    .translate(Coord::new(1, 93))
                    .into_iter(),
            );
        }

        if let Some(uname) = read_uname() {
            display.draw(
                ProFont9Point::render_str(uname.trim())
                    .with_stroke(Some(Color::Black))
                    .with_fill(Some(Color::White))
                    .translate(Coord::new(1, 84))
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

fn read_cpu_temp() -> Result<f64, io::Error> {
    fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")?
        .trim()
        .parse::<i32>()
        .map(|temp| temp as f64 / 1000.)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
}

fn read_uptime() -> Option<String> {
    Command::new("uptime")
        .arg("-p")
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
