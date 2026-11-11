//! The I/O mailbox card (logisim/notes/iod-requirements.md, "Target design",
//! I10-I16): the CPU side of the link to the I/O processor (an ESP32, see
//! [`crate::iop`]).
//!
//! The model follows the Logisim module (circuits/iod/iod.circ) chip by
//! chip, at the level of edges and levels. Each `pub fn` below is one thing
//! a chip does. `hbc2 vectors iod` drives these same functions from pin
//! rows to test the Logisim circuit, so the circuit and the emulator share
//! this code as their reference.
//!
//! - **Decode** (74HCT138): G1 = IE, /G2A = A7, /G2B = A6, A = A0, B = RW,
//!   C = A1. Ports `0x00-0x3F` with A1 = 0 answer, A2-A5 are ignored.
//! - **TX** (CPU -> MCU): OUT TX loads the 74HCT165 and sets `tx_full` (74
//!   /PRE). The MCU shifts it out and clears `tx_full` when MBX_SEL rises.
//! - **RX** (MCU -> CPU): the MCU shifts a byte into the RX 595 and pulses
//!   RCLK_RX (byte latched, `rx_full` set). IN RX drives it; `rx_full`
//!   clears in the step after the IN (`rx_clr`, a 74 on `clk`).
//! - **EVT** (MCU -> CPU, interrupt): the MCU shifts data then port into the
//!   two EVT 595s and pulses RCLK_EVT. `evt_s` (INT) samples `evt_pend` on
//!   `clk`. In the vector step (VEC = INR one step late) the 595s drive
//!   A0-A7 = port and `bus_data` = data; `evt_pend` clears in the step after.
//! - **STATUS** (74HCT244): bit 0 RX_AVAIL, bit 1 TX_FREE, bit 2 EVT_PEND.
//!
//! One CPU clock cycle of the emulator is one 500 ns step of the hardware.

/// What the 74HCT138 selects in a step (I10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Select {
    #[default]
    None,
    /// OUT, port = 0 (mod 4) in `0x00-0x3F`.
    Tx,
    /// IN, port = 0 (mod 4).
    Rx,
    /// IN, port = 1 (mod 4).
    Status,
}

/// The 74HCT138. `read` = RW (1 for IN).
#[inline(always)]
pub fn decode(ie: bool, read: bool, port: u8) -> Select {
    if !ie || port & 0xC0 != 0 {
        return Select::None;
    }
    match ((port >> 1) & 1, read, port & 1) {
        (0, false, 0) => Select::Tx,
        (0, true, 0) => Select::Rx,
        (0, true, 1) => Select::Status,
        _ => Select::None,
    }
}

/// STATUS bits (I11).
pub const RX_AVAIL: u8 = 0x01;
pub const TX_FREE: u8 = 0x02;
pub const EVT_PEND: u8 = 0x04;

/// The card's state. [`Mailbox::new`] is the state a step after a reset
/// (I16): every flag clear, the clears released, the 595 / 165 contents at 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mailbox {
    // ---- the 74HCT74 flip-flops
    /// A byte waits in TX for the MCU.
    pub tx_full: bool,
    /// A byte waits in RX for the CPU.
    pub rx_full: bool,
    /// An event waits in the EVT latches.
    pub evt_pend: bool,
    /// `rx_clr` (low = clearing RX): D = /Y2 on `clk`.
    pub rx_clr_n: bool,
    /// `evt_clr` (low = clearing EVT): D = /VEC on `clk`.
    pub evt_clr_n: bool,
    /// VEC: D = INR on `clk`. High in the vector step.
    pub vec: bool,
    /// `evt_s` = INT: D = EVT on `clk`, /CLR = /VEC.
    pub evt_s: bool,
    // ---- the shift registers
    /// The TX 74HCT165.
    pub tx: u8,
    /// The RX 595: shift register and storage register.
    pub rx_shift: u8,
    pub rx: u8,
    /// The EVT port 595 (its QH' feeds the EVT data 595).
    pub port_shift: u8,
    pub port: u8,
    /// The EVT data 595.
    pub data_shift: u8,
    pub data: u8,
    // ---- emulator bookkeeping (not a chip)
    /// What the decode selected in the step now running (for `rx_clr`).
    step: Select,
    /// The MCU's MOSI level, for the SCK edges of a TX read.
    mosi: bool,
}

impl Default for Mailbox {
    fn default() -> Self {
        Mailbox::new()
    }
}

impl Mailbox {
    pub fn new() -> Mailbox {
        Mailbox {
            tx_full: false,
            rx_full: false,
            evt_pend: false,
            rx_clr_n: true,
            evt_clr_n: true,
            vec: false,
            evt_s: false,
            tx: 0,
            rx_shift: 0,
            rx: 0,
            port_shift: 0,
            port: 0,
            data_shift: 0,
            data: 0,
            step: Select::None,
            mosi: false,
        }
    }

    // ------------------------------------------------------------------
    // Chip actions, one per edge or level (shared with `hbc2 vectors iod`)
    // ------------------------------------------------------------------

    /// The `clk` rising edge at the end of a step. `step` is what the decode
    /// selected during the step, `inr` the INR line during it.
    #[inline(always)]
    pub fn clk_rise(&mut self, step: Select, inr: bool) {
        let vec_n_old = !self.vec;
        self.rx_clr_n = step != Select::Rx;
        self.evt_clr_n = vec_n_old;
        self.vec = inr;
        // evt_s's /CLR was low (VEC high) until this edge.
        self.evt_s = vec_n_old && self.evt_pend;
    }

    /// MBX_SEL rising: the MCU has read TX (CLK of the TX flip-flop, D = 0).
    pub fn mbx_sel_rise(&mut self) {
        self.tx_full = false;
    }

    /// RCLK_RX rising: the RX 595 latches its shift register, RX full is set.
    pub fn rclk_rx_rise(&mut self) {
        self.rx_full = true;
        self.rx = self.rx_shift;
    }

    /// RCLK_EVT rising: both EVT 595s latch, EVT pending is set.
    pub fn rclk_evt_rise(&mut self) {
        self.evt_pend = true;
        self.port = self.port_shift;
        self.data = self.data_shift;
    }

    /// SCK rising, with MOSI and the 165's state before the edge. The three
    /// 595s always shift; the 165 shifts (SER = 0) only when its clock is
    /// enabled (MBX_SEL low) and it isn't loading (TX strobe high).
    pub fn sck_rise(&mut self, mosi: bool, shift_tx: bool) {
        if shift_tx {
            self.tx <<= 1;
        }
        let link = self.port_shift >> 7;
        self.rx_shift = self.rx_shift << 1 | mosi as u8;
        self.data_shift = self.data_shift << 1 | link;
        self.port_shift = self.port_shift << 1 | mosi as u8;
    }

    /// The asynchronous presets and clears, after any edge or input change:
    /// `rst_n` low (I16), the TX strobe low (`tx_load` = the byte on
    /// `bus_data`: the 165 loads, TX full is set), `rx_clr`, `evt_clr`, VEC.
    #[inline(always)]
    pub fn settle(&mut self, rst_n: bool, tx_load: Option<u8>) {
        if !rst_n {
            self.tx_full = false;
            self.rx_clr_n = false;
            self.evt_clr_n = false;
            self.vec = false;
        }
        if let Some(byte) = tx_load {
            self.tx_full = true;
            self.tx = byte;
        }
        if !self.rx_clr_n {
            self.rx_full = false;
        }
        if !self.evt_clr_n {
            self.evt_pend = false;
        }
        if self.vec {
            self.evt_s = false;
        }
    }

    // ------------------------------------------------------------------
    // Outputs
    // ------------------------------------------------------------------

    /// The STATUS byte (I11).
    #[inline(always)]
    pub fn status(&self) -> u8 {
        let bit = |on: bool, b: u8| if on { b } else { 0 };
        bit(self.rx_full, RX_AVAIL) | bit(!self.tx_full, TX_FREE) | bit(self.evt_pend, EVT_PEND)
    }

    /// What the card drives on `bus_data` in a step that selects `sel`
    /// (None = nothing). A selected IN in the vector step would be two
    /// drivers; the CPU never does that.
    pub fn bus_data(&self, sel: Select) -> Option<u8> {
        match sel {
            Select::Rx => Some(self.rx),
            Select::Status => Some(self.status()),
            _ if self.vec => Some(self.data),
            _ => None,
        }
    }

    /// A0-A7 in the vector step (A8-A15 are pulled down on the board).
    pub fn addr_out(&self) -> Option<u8> {
        self.vec.then_some(self.port)
    }

    /// The INT line.
    #[inline(always)]
    pub fn int(&self) -> bool {
        self.evt_s
    }

    /// Nothing changes at the next `clk` edge of a step with no I/O and INR
    /// low: the clears are released, VEC is low and INT follows EVT.
    #[inline(always)]
    pub fn settled(&self) -> bool {
        self.rx_clr_n && self.evt_clr_n && !self.vec && self.evt_s == self.evt_pend && self.step == Select::None
    }

    // ------------------------------------------------------------------
    // The CPU side, one step at a time (the emulator)
    // ------------------------------------------------------------------

    /// IN in the step now running: the byte on `bus_data` (0xFF when the
    /// port is empty, I10).
    pub fn cpu_in(&mut self, port: u8) -> u8 {
        let sel = decode(true, true, port);
        self.step = sel;
        self.bus_data(sel).unwrap_or(0xFF)
    }

    /// OUT in the step now running: `clk_wr_n` low loads TX if selected.
    pub fn cpu_out(&mut self, port: u8, value: u8) {
        let sel = decode(true, false, port);
        self.step = sel;
        if sel == Select::Tx {
            self.settle(true, Some(value));
            self.settle(true, None);
        }
    }

    /// The `clk` edge at the end of the step; `inr` = the INR line in it.
    #[inline(always)]
    pub fn clock(&mut self, inr: bool) {
        let step = std::mem::take(&mut self.step);
        self.clk_rise(step, inr);
        self.settle(true, None);
    }

    // ------------------------------------------------------------------
    // The MCU side (between two `clk` edges: the sides are asynchronous)
    // ------------------------------------------------------------------

    fn shift_out(&mut self, byte: u8) {
        for b in (0..8).rev() {
            self.mosi = (byte >> b) & 1 != 0;
            self.sck_rise(self.mosi, false);
        }
    }

    /// Writes RX: 8 bits, then RCLK_RX. Lost if `rx_clr` is low (I15).
    pub fn mcu_write_rx(&mut self, byte: u8) {
        self.shift_out(byte);
        self.rclk_rx_rise();
        self.settle(true, None);
    }

    /// Writes an event: the data byte, then the port byte, then RCLK_EVT.
    /// Lost if `evt_clr` is low (I15).
    pub fn mcu_write_evt(&mut self, port: u8, data: u8) {
        self.shift_out(data);
        self.shift_out(port);
        self.rclk_evt_rise();
        self.settle(true, None);
    }

    /// Reads TX: MBX_SEL low (QH = bit 7), 7 SCK edges, MBX_SEL high (TX
    /// full clears). Returns the byte.
    pub fn mcu_read_tx(&mut self) -> u8 {
        let mut byte = 0;
        for _ in 0..7 {
            byte = byte << 1 | self.tx >> 7;
            self.sck_rise(self.mosi, true);
        }
        byte = byte << 1 | self.tx >> 7;
        self.mbx_sel_rise();
        self.settle(true, None);
        byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_follows_i10() {
        for port in 0..=255u8 {
            let low = port < 0x40 && port & 2 == 0;
            assert_eq!(decode(true, false, port) == Select::Tx, low && port & 1 == 0);
            assert_eq!(decode(true, true, port) == Select::Rx, low && port & 1 == 0);
            assert_eq!(decode(true, true, port) == Select::Status, low && port & 1 == 1);
            assert_eq!(decode(false, true, port), Select::None);
        }
    }

    #[test]
    fn rx_written_during_the_clear_is_lost() {
        let mut m = Mailbox::new();
        m.mcu_write_rx(0x5A);
        assert_eq!(m.cpu_in(0x00), 0x5A);
        m.clock(false); // the clear step starts
        assert!(!m.rx_full);
        m.mcu_write_rx(0x33); // too early
        assert!(!m.rx_full);
        m.clock(false);
        m.mcu_write_rx(0x34);
        assert!(m.rx_full);
        assert_eq!(m.cpu_in(0x3C), 0x34);
    }

    #[test]
    fn tx_round_trip() {
        let mut m = Mailbox::new();
        assert_eq!(m.cpu_in(0x01) & TX_FREE, TX_FREE);
        m.cpu_out(0x20, 0xA7);
        m.clock(false);
        assert!(m.tx_full);
        assert_eq!(m.mcu_read_tx(), 0xA7);
        assert!(!m.tx_full);
    }

    #[test]
    fn event_handshake() {
        let mut m = Mailbox::new();
        m.mcu_write_evt(0xF3, 0xC4);
        assert!(!m.int());
        m.clock(false);
        assert!(m.int());
        m.clock(true); // the CPU raised INR in this step
        assert!(!m.int() && m.vec);
        assert_eq!((m.addr_out(), m.bus_data(Select::None)), (Some(0xF3), Some(0xC4)));
        m.clock(false); // end of the vector step: evt_clr low
        assert!(!m.evt_pend && !m.int());
        m.clock(false);
        assert!(m.settled());
    }
}
