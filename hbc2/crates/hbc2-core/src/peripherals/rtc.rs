//! The HBC-2 Real-Time Clock.
//!
//! Ports: YEAR (years since 1900), MONTH, DAY, HOUR, MINUTE, SECOND, CMD.
//!
//! * Every second, the date and time are put on the ports and an interrupt is
//!   sent on the first port.
//! * CMD = GET_TIME puts the date and time on the ports.
//! * CMD = SET_TIME reads the date and time from the ports. The YEAR port
//!   then holds 1 on success, 0 if the date or time is invalid.
//!
//! CMD is reset to NOP once the command is executed.
//!
//! The clock follows a reference time: the host's local time when the
//! emulator syncs it (`Rtc::sync`), else the time reported by `advance_time`.
//! SET_TIME does not stop that: it shifts the clock by a fixed offset.

use super::Peripheral;
use crate::board::Board;

pub const YEAR_0: u16 = 1900;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const SECONDS_PER_DAY: i64 = 86_400;
/// Seconds from 1900-01-01 to 2156-01-01: the YEAR port is 8 bits, so the
/// clock wraps from 2155-12-31 23:59:59 back to 1900-01-01 00:00:00.
const SPAN: i64 = 93_502 * SECONDS_PER_DAY;

pub mod port {
    pub const YEAR: u8 = 0;
    pub const MONTH: u8 = 1;
    pub const DAY: u8 = 2;
    pub const HOUR: u8 = 3;
    pub const MINUTE: u8 = 4;
    pub const SECOND: u8 = 5;
    pub const CMD: u8 = 6;
}

pub mod command {
    pub const NOP: u8 = 0;
    pub const GET_TIME: u8 = 1;
    pub const SET_TIME: u8 = 2;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl DateTime {
    /// Value at power-on, until the emulator syncs the clock with the host.
    pub const DEFAULT: DateTime = DateTime {
        year: 2000,
        month: 1,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
    };

    pub fn is_valid(&self) -> bool {
        (1..=12).contains(&self.month)
            && self.day >= 1
            && self.day <= days_in_month(self.year, self.month)
            && self.hour < 24
            && self.minute < 60
            && self.second < 60
    }

    /// Seconds since 1900-01-01 00:00:00.
    fn to_seconds(self) -> i64 {
        let days = days_from_civil(self.year as i64, self.month as i64, self.day as i64)
            - days_from_civil(YEAR_0 as i64, 1, 1);
        days * SECONDS_PER_DAY
            + self.hour as i64 * 3600
            + self.minute as i64 * 60
            + self.second as i64
    }

    /// Inverse of `to_seconds`.
    fn from_seconds(seconds: i64) -> DateTime {
        let days = seconds.div_euclid(SECONDS_PER_DAY) + days_from_civil(YEAR_0 as i64, 1, 1);
        let time = seconds.rem_euclid(SECONDS_PER_DAY);
        let (year, month, day) = civil_from_days(days);
        DateTime {
            year: year as u16,
            month: month as u8,
            day: day as u8,
            hour: (time / 3600) as u8,
            minute: (time / 60 % 60) as u8,
            second: (time % 60) as u8,
        }
    }
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => {
            29
        }
        2 => 28,
        _ => 31,
    }
}

/// Days since 1970-01-01 of a Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Inverse of `days_from_civil`: (year, month, day).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + (month <= 2) as i64;
    (year, month, day)
}

#[derive(Clone, Debug)]
pub struct Rtc {
    first_port: Option<u8>,
    /// Reference time, in seconds since 1900-01-01.
    reference: i64,
    /// Seconds added to the reference by SET_TIME.
    offset: i64,
    /// Nanoseconds accumulated towards the next second (`advance_time`).
    elapsed: u64,
}

impl Default for Rtc {
    fn default() -> Self {
        Rtc::new()
    }
}

impl Rtc {
    pub fn new() -> Rtc {
        Rtc {
            first_port: None,
            reference: DateTime::DEFAULT.to_seconds(),
            offset: 0,
            elapsed: 0,
        }
    }

    pub fn now(&self) -> DateTime {
        DateTime::from_seconds(self.seconds())
    }

    fn seconds(&self) -> i64 {
        (self.reference + self.offset).rem_euclid(SPAN)
    }

    /// Sets the reference time to the host's time. If the clock's second
    /// changes, the date and time are put on the ports, and an interrupt is
    /// sent when `interrupt` is set. Returns true if the clock changed.
    pub fn sync(&mut self, host: DateTime, interrupt: bool, board: &mut Board) -> bool {
        self.elapsed = 0;
        self.set_reference(host.to_seconds(), interrupt, board)
    }

    fn set_reference(&mut self, reference: i64, interrupt: bool, board: &mut Board) -> bool {
        let before = self.seconds();
        self.reference = reference;
        if self.seconds() == before {
            return false;
        }
        if let Some(first) = self.first_port {
            self.put_on_ports(first, board);
            if interrupt {
                board.iod.trigger_interrupt(first);
            }
        }
        true
    }

    /// Does what the RTC does every second, at once: puts the date and time
    /// on the ports and sends the interrupt. Returns false if the RTC is not
    /// connected or the interrupt queue is full.
    pub fn interrupt(&self, board: &mut Board) -> bool {
        let Some(first) = self.first_port else {
            return false;
        };
        self.put_on_ports(first, board);
        board.iod.trigger_interrupt(first)
    }

    fn put_on_ports(&self, first: u8, board: &mut Board) {
        let now = self.now();
        let iod = &mut board.iod;
        iod.set_port(first + port::YEAR, (now.year - YEAR_0) as u8);
        iod.set_port(first + port::MONTH, now.month);
        iod.set_port(first + port::DAY, now.day);
        iod.set_port(first + port::HOUR, now.hour);
        iod.set_port(first + port::MINUTE, now.minute);
        iod.set_port(first + port::SECOND, now.second);
    }
}

impl Peripheral for Rtc {
    const DEVICE_ID: u8 = 0x95;
    const PORT_COUNT: u8 = 7;
    const NAME: &'static str = "Real-Time Clock";

    fn first_port(&self) -> Option<u8> {
        self.first_port
    }

    fn power_on(&mut self, first_port: u8, _board: &mut Board) {
        *self = Rtc::new();
        self.first_port = Some(first_port);
    }

    fn port_written(&mut self, offset: u8, board: &mut Board) {
        let Some(first) = self.first_port else {
            return;
        };
        if offset != port::CMD {
            return;
        }
        match board.iod.port(first + port::CMD) {
            command::GET_TIME => self.put_on_ports(first, board),
            command::SET_TIME => {
                let iod = &board.iod;
                let requested = DateTime {
                    year: YEAR_0 + iod.port(first + port::YEAR) as u16,
                    month: iod.port(first + port::MONTH),
                    day: iod.port(first + port::DAY),
                    hour: iod.port(first + port::HOUR),
                    minute: iod.port(first + port::MINUTE),
                    second: iod.port(first + port::SECOND),
                };
                let valid = requested.is_valid();
                if valid {
                    self.offset = requested.to_seconds() - self.reference;
                }
                board.iod.set_port(first + port::YEAR, valid as u8);
            }
            _ => return,
        }
        board.iod.set_port(first + port::CMD, command::NOP);
    }

    fn advance_time(&mut self, nanos: u64, board: &mut Board) {
        if self.first_port.is_none() {
            return;
        }
        self.elapsed += nanos;
        let seconds = self.elapsed / NANOS_PER_SECOND;
        if seconds > 0 {
            self.elapsed %= NANOS_PER_SECOND;
            self.set_reference(self.reference + seconds as i64, true, board);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> DateTime {
        DateTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    #[test]
    fn seconds_round_trip() {
        assert_eq!(DateTime::from_seconds(0), date(1900, 1, 1, 0, 0, 0));
        for d in [
            DateTime::DEFAULT,
            date(2024, 2, 29, 23, 59, 59),
            date(2026, 10, 25, 2, 30, 0),
        ] {
            assert_eq!(DateTime::from_seconds(d.to_seconds()), d);
        }
        assert_eq!(date(2155, 12, 31, 23, 59, 59).to_seconds(), SPAN - 1);
    }

    #[test]
    fn manual_interrupt_puts_the_time_on_the_ports() {
        let mut board = Board::default();
        let mut rtc = Rtc::new();
        assert!(!rtc.interrupt(&mut board), "not connected");
        rtc.first_port = Some(3);
        rtc.sync(date(2026, 10, 28, 14, 30, 7), false, &mut board);
        assert!(rtc.interrupt(&mut board));
        assert_eq!(board.iod.port(3 + port::SECOND), 7);
        let queued: Vec<_> = board.iod.queue().collect();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].port, 3);
        assert_eq!(queued[0].data, (2026 - YEAR_0) as u8);
    }

    #[test]
    fn sync_follows_the_host_and_keeps_the_set_time_offset() {
        let mut board = Board::default();
        let mut rtc = Rtc::new();
        let host = date(2026, 10, 28, 14, 30, 0);
        assert!(rtc.sync(host, false, &mut board));
        assert_eq!(rtc.now(), host);
        assert!(!rtc.sync(host, false, &mut board), "same second: no change");

        rtc.offset = 3600; // as if SET_TIME asked for one hour later
        rtc.sync(date(2026, 10, 28, 14, 30, 5), false, &mut board);
        assert_eq!(rtc.now(), date(2026, 10, 28, 15, 30, 5));
    }
}
