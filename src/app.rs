use nix::sys::utsname::UtsName;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::time::{Duration, Instant};

use systemstat::{Memory, System, Platform};
use nix::sys::utsname::uname;
use rs_release::get_os_release;

use crate::system::Uptime;

pub struct State {
    hi_count: usize,
    interface: String,
    pub ip: Option<systemstat::Ipv4Addr>,
    pub os_name: String,
    pub uname: UtsName,
    pub memory: Option<Memory>,
    pub uptime: Uptime,
    pub hellos: HashMap<IpAddr, Instant>,
    max_age: Duration,
    system: System,
}

impl State {
    pub fn new(
        hi_count: usize,
        interface: String,
        max_age: Duration
    ) -> Self {
        let system = System::new();
        let os_name = get_os_release()
            .ok()
            .and_then(|mut hash| hash.remove("NAME"))
            .unwrap_or_else(|| "Unknown".to_string());

        State {
            hi_count,
            ip: get_interface_ip(&system, &interface),
            os_name,
            uname: uname(),
            memory: system.memory().ok(),
            uptime: system
                .uptime()
                .ok()
                .map(|uptime| Uptime::new(uptime.as_secs()))
                .unwrap_or_default(),
            hellos: HashMap::new(),
            max_age,
            system,
            interface,
        }
    }

    pub fn load(
        save_path: &Path,
        interface: String,
        max_age: Duration,
    ) -> io::Result<Self> {
        let hi_count = match Self::load_hi_count(save_path) {
            Ok(hi_count) => hi_count,
            Err(err) => match err.kind() {
                io::ErrorKind::NotFound => 0,
                _ => return Err(err),
            },
        };

        println!("Loaded state with hi count {}", hi_count);
        Ok(Self::new(
            hi_count, interface, max_age,
        ))
    }

    pub fn inc_hi_count(&mut self, from: IpAddr) {
        let now = Instant::now();

        match self.hellos.get(&from).map(|instant| now - *instant) {
            Some(age) if age > self.max_age => self.inc_hi_count_impl(from, now),
            None => self.inc_hi_count_impl(from, now),
            _ => (),
        }
    }

    fn inc_hi_count_impl(&mut self, from: IpAddr, now: Instant) {
        self.hellos.insert(from, now);
        self.hi_count += 1;
    }

    pub fn hi_count(&self) -> usize {
        self.hi_count
    }

    fn load_hi_count(path: &Path) -> io::Result<usize> {
        let string_count = std::fs::read_to_string(path)?;
        usize::from_str(string_count.trim()).or(Ok(0))
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

    pub fn refresh(&mut self) {
        self.ip = get_interface_ip(&self.system, &self.interface);
        self.memory = self.system.memory().ok();
        self.uptime = self.system
            .uptime()
            .ok()
            .map(|uptime| Uptime::new(uptime.as_secs()))
            .unwrap_or_default();
    }
}

fn get_interface_ip(system: &System, interface: &str) -> Option<systemstat::Ipv4Addr> {
    if let Some(network) = system
        .networks()
        .ok()
        .and_then(|mut info| info.remove(interface))
    {
        for addr in network.addrs {
            match addr.addr {
                systemstat::IpAddr::V4(addr) => return Some(addr),
                _ => (),
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::utsname::uname;
    use std::net::{IpAddr, Ipv4Addr};
    use std::thread;

    fn test_state(ip: Ipv4Addr) -> State {
        State::new(
            0,
            Some(ip),
            "Test OS".to_string(),
            uname(),
            None,
            Uptime::default(),
            Duration::from_millis(100),
        )
    }

    #[test]
    fn test_inc_hello_dedup() {
        let localhost = Ipv4Addr::new(127, 0, 0, 1);
        let mut state = test_state(localhost);

        state.inc_hi_count(IpAddr::V4(localhost));
        state.inc_hi_count(IpAddr::V4(localhost));
        state.inc_hi_count(IpAddr::V4(localhost));

        assert_eq!(state.hi_count(), 1);
    }

    #[test]
    fn test_inc_hello_expiration() {
        let localhost = Ipv4Addr::new(127, 0, 0, 1);
        let mut state = test_state(localhost);

        state.inc_hi_count(IpAddr::V4(localhost));
        state.inc_hi_count(IpAddr::V4(localhost));
        thread::sleep(Duration::from_millis(101));
        state.inc_hi_count(IpAddr::V4(localhost));
        state.inc_hi_count(IpAddr::V4(localhost));

        assert_eq!(state.hi_count(), 2);
    }
}
