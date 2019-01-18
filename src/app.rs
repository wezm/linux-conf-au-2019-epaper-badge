use nix::sys::utsname::UtsName;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant};
use systemstat::{Ipv4Addr, Memory};

use crate::system::Uptime;

// #[derive(Debug)]
pub struct State {
    hi_count: usize,
    pub ip: Option<Ipv4Addr>,
    pub os_name: String,
    pub uname: UtsName,
    pub memory: Option<Memory>,
    pub uptime: Uptime,
    pub hellos: HashMap<IpAddr, Instant>,
}

impl State {
    pub fn new(
        hi_count: usize,
        ip: Option<Ipv4Addr>,
        os_name: String,
        uname: UtsName,
        memory: Option<Memory>,
        uptime: Uptime,
    ) -> Self {
        State {
            hi_count,
            ip,
            os_name,
            uname,
            memory,
            uptime,
            hellos: HashMap::new(),
        }
    }

    pub fn load(
        save_path: &Path,
        ip: Option<Ipv4Addr>,
        os_name: String,
        uname: UtsName,
        memory: Option<Memory>,
        uptime: Uptime,
    ) -> io::Result<Self> {
        let hi_count = match Self::load_hi_count(save_path) {
            Ok(hi_count) => hi_count,
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => 0,
                _ => return Err(err),
            },
        };

        Ok(Self::new(hi_count, ip, os_name, uname, memory, uptime))
    }

    pub fn inc_hi_count(&mut self, from: IpAddr) {
        if self.hellos.insert(from, Instant::now()).is_none() {
            self.hi_count += 1;
        }
    }

    pub fn hi_count(&self) -> usize {
        self.hi_count
    }

    fn load_hi_count(path: &Path) -> io::Result<usize> {
        let string_count = std::fs::read_to_string(path)?;
        usize::from_str(&string_count).or(Ok(0))
    }

    pub fn save_hi_count(&self, path: &Path) -> io::Result<()> {
        let tmp_filename = path
            .file_name()
            .map(|file_name| {
                let mut tmp_name = file_name.to_os_string();
                tmp_name.push(".tmp");
                tmp_name
            })
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "file_name is None"))?;
        let tmp_path = path.with_file_name(tmp_filename);
        fs::write(&tmp_path, self.hi_count().to_string())?;

        std::fs::rename(tmp_path, path)?;

        println!("Saved hi count");

        Ok(())
    }
}
