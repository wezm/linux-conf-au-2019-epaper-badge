use std::fmt;

const ONE_DAY: u64 = 24 * 60 * 60;
const ONE_HOUR: u64 = 60 * 60;

#[derive(Debug, Copy, Clone, Default)]
pub struct Uptime(u64);

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
