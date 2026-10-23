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

use super::Peripheral;
use crate::board::Board;

pub const YEAR_0: u16 = 1900;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

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
    /// Default value at power-on.
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

    fn add_second(&mut self) {
        self.second += 1;
        if self.second < 60 {
            return;
        }
        self.second = 0;
        self.minute += 1;
        if self.minute < 60 {
            return;
        }
        self.minute = 0;
        self.hour += 1;
        if self.hour < 24 {
            return;
        }
        self.hour = 0;
        self.day += 1;
        if self.day <= days_in_month(self.year, self.month) {
            return;
        }
        self.day = 1;
        self.month += 1;
        if self.month <= 12 {
            return;
        }
        self.month = 1;
        // The YEAR port is 8 bits: wrap from 2155 back to 1900.
        self.year = if self.year >= YEAR_0 + 255 {
            YEAR_0
        } else {
            self.year + 1
        };
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

#[derive(Clone, Debug)]
pub struct Rtc {
    first_port: Option<u8>,
    now: DateTime,
    /// Nanoseconds accumulated towards the next second.
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
            now: DateTime::DEFAULT,
            elapsed: 0,
        }
    }

    pub fn now(&self) -> DateTime {
        self.now
    }

    fn put_on_ports(&self, first: u8, board: &mut Board) {
        let iod = &mut board.iod;
        iod.set_port(first + port::YEAR, (self.now.year - YEAR_0) as u8);
        iod.set_port(first + port::MONTH, self.now.month);
        iod.set_port(first + port::DAY, self.now.day);
        iod.set_port(first + port::HOUR, self.now.hour);
        iod.set_port(first + port::MINUTE, self.now.minute);
        iod.set_port(first + port::SECOND, self.now.second);
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
                    self.now = requested;
                    self.elapsed = 0;
                }
                board.iod.set_port(first + port::YEAR, valid as u8);
            }
            _ => return,
        }
        board.iod.set_port(first + port::CMD, command::NOP);
    }

    fn advance_time(&mut self, nanos: u64, board: &mut Board) {
        let Some(first) = self.first_port else {
            return;
        };
        self.elapsed += nanos;
        if self.elapsed < NANOS_PER_SECOND {
            return;
        }
        while self.elapsed >= NANOS_PER_SECOND {
            self.elapsed -= NANOS_PER_SECOND;
            self.now.add_second();
        }
        self.put_on_ports(first, board);
        board.iod.trigger_interrupt(first);
    }
}
