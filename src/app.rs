use nix::sys::utsname::UtsName;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::Path;
use std::time::{Duration, Instant};
use systemstat::{Ipv4Addr, Memory};

use crate::system::Uptime;

// #[derive(Debug)]
pub struct State {
    pub ip: Option<Ipv4Addr>,
    pub os_name: String,
    pub uname: UtsName,
    pub memory: Option<Memory>,
    pub uptime: Uptime,
    pub hellos: HashMap<IpAddr, Instant>,
}

impl State {
    pub fn new(
        ip: Option<Ipv4Addr>,
        os_name: String,
        uname: UtsName,
        memory: Option<Memory>,
        uptime: Uptime,
    ) -> Self {
        State {
            ip,
            os_name,
            uname,
            memory,
            uptime,
            hellos: HashMap::new(),
        }
    }

    pub fn inc_hi_count(&mut self, from: IpAddr) {
        self.hellos.insert(from, Instant::now());
    }

    pub fn hi_count(&self) -> usize {
        self.hellos.len()
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

        std::fs::rename(tmp_path, path)
    }
}
