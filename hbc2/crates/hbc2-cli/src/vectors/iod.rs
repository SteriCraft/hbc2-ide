//! The I/O mailbox card (logisim/notes/iod-requirements.md I10-I16,
//! iod-decisions.md).
//!
//! `iod`: four sequences, each starting with a reset (the flip-flops power
//! up unknown). The rows drive the card's pins; [`Card::row`] finds the
//! edges, as Logisim does, and calls the emulator's own chip functions
//! (`hbc2_core::mailbox`) in the same order as the card's chips act. The
//! expected outputs are what those functions leave.
//!
//! 1. Every port 0-255, IN and OUT, IE on and off: who drives `bus_data`,
//!    what loads TX.
//! 2. Both handshakes, 40 rounds, MISO bit by bit, STATUS at each stage, and
//!    the 1 us rule broken at random: an RX written during the clear is lost.
//! 3. 30 events: INT, the vector step (A0-A7 = port, `bus_data` = data, INT
//!    low), EVT cleared after it; an event written during that clear is lost.
//! 4. Reset with every flag set: all clear, the latches keep their bytes.
//!
//! One CPU step = 4 rows: the `clk` edge (with the old step's inputs), the
//! new inputs, `clk` low with `clk_wr_n` low, `clk_wr_n` high. The MCU's
//! actions fall between rows: the two sides are asynchronous.

use super::{Out, Row, Rng, SeqModule};
use hbc2_core::mailbox::{Mailbox, Select, decode};

const IOD_IN: &[(&str, u32)] = &[
    ("iod_bus_in", 8),
    ("iod_addr_in", 8),
    ("iod_ie", 1),
    ("iod_rw", 1),
    ("iod_inr", 1),
    ("clk", 1),
    ("clk_wr_n", 1),
    ("rst_n", 1),
    ("iod_sck", 1),
    ("iod_mosi", 1),
    ("iod_rclk_rx", 1),
    ("iod_rclk_evt", 1),
    ("iod_mbx_sel", 1),
];
const IOD_OUT: &[(&str, u32)] = &[
    ("iod_bus_out", 8),
    ("iod_addr_out", 8),
    ("iod_int", 1),
    ("iod_miso", 1),
    ("iod_mcu_tx_full", 1),
    ("iod_mcu_rx_full", 1),
    ("iod_mcu_evt_pend", 1),
    ("iod_mcu_rst_n", 1),
];
const IOD_BUSES: &[&str] = &["iod_bus_out", "iod_addr_out", "iod_miso"];

// Input indexes, in IOD_IN order.
const BUS_IN: usize = 0;
const ADDR_IN: usize = 1;
const IE: usize = 2;
const RW: usize = 3;
const INR: usize = 4;
const CLK: usize = 5;
const CLK_WR_N: usize = 6;
const RST_N: usize = 7;
const SCK: usize = 8;
const MOSI: usize = 9;
const RCLK_RX: usize = 10;
const RCLK_EVT: usize = 11;
const MBX_SEL: usize = 12;

type Pins = [u32; 13];

fn select(p: &Pins) -> Select {
    decode(p[IE] != 0, p[RW] != 0, p[ADDR_IN] as u8)
}

/// The TX strobe: /Y0 OR `clk_wr_n` (74HCT32).
fn tx_wr_n(p: &Pins) -> bool {
    select(p) != Select::Tx || p[CLK_WR_N] != 0
}

struct Card {
    pins: Pins,
    m: Mailbox,
    /// `evt_s` has had a `clk` edge since power-up (it has no reset of its own).
    int_known: bool,
    rows: Vec<Row>,
    rng: Rng,
}

impl Card {
    /// Powered up, `rst_n` low, then two steps in reset.
    fn new(seed: u64) -> Card {
        let mut pins = [0; 13];
        pins[CLK_WR_N] = 1;
        pins[MBX_SEL] = 1;
        let mut k = Card { pins, m: Mailbox::new(), int_known: false, rows: Vec::new(), rng: Rng(seed) };
        k.row(&[(RST_N, 0)]);
        k.step(Step { rst_n: 0, ..Step::IDLE });
        k.step(Step { rst_n: 0, ..Step::IDLE });
        k
    }

    fn byte(&mut self) -> u8 {
        self.rng.byte() as u8
    }

    /// One row: the edges first (sampling the pins from before the row),
    /// then the levels, then the outputs.
    fn row(&mut self, changes: &[(usize, u32)]) {
        let old = self.pins;
        for &(i, v) in changes {
            self.pins[i] = v;
        }
        let new = self.pins;
        let rose = |i: usize| old[i] == 0 && new[i] == 1;
        if rose(CLK) {
            self.m.clk_rise(select(&old), old[INR] != 0);
            self.int_known = true;
        }
        if rose(MBX_SEL) {
            self.m.mbx_sel_rise();
        }
        if rose(RCLK_RX) {
            self.m.rclk_rx_rise();
        }
        if rose(RCLK_EVT) {
            self.m.rclk_evt_rise();
        }
        if rose(SCK) {
            self.m.sck_rise(old[MOSI] != 0, tx_wr_n(&old) && old[MBX_SEL] == 0);
        }
        let load = (!tx_wr_n(&new)).then_some(new[BUS_IN] as u8);
        self.m.settle(new[RST_N] != 0, load);
        let outs = self.outputs();
        self.rows.push((new.to_vec(), outs));
    }

    fn outputs(&self) -> Vec<Out> {
        let m = &self.m;
        let sel = select(&self.pins);
        let drivers = (sel == Select::Rx) as u32 + (sel == Select::Status) as u32 + m.vec as u32;
        assert!(drivers <= 1, "two drivers on bus_data in a test");
        let v = |b: bool| Out::V(b as u32);
        vec![
            m.bus_data(sel).map_or(Out::Z, |d| Out::V(d as u32)),
            m.addr_out().map_or(Out::Z, |a| Out::V(a as u32)),
            if self.int_known { v(m.int()) } else { Out::X },
            if self.pins[MBX_SEL] != 0 { Out::Z } else { Out::V((m.tx >> 7) as u32) },
            v(m.tx_full),
            v(m.rx_full),
            v(m.evt_pend),
            Out::V(self.pins[RST_N]),
        ]
    }

    // ---- the CPU

    fn step(&mut self, s: Step) {
        let addr = match s.port {
            Some(p) => p,
            None => self.byte(),
        };
        let data = match s.data {
            Some(d) => d,
            None => self.byte(),
        };
        self.row(&[(CLK, 1)]);
        self.row(&[
            (IE, s.ie),
            (RW, s.rw),
            (ADDR_IN, addr as u32),
            (INR, s.inr),
            (RST_N, s.rst_n),
            (BUS_IN, data as u32),
        ]);
        self.row(&[(CLK, 0), (CLK_WR_N, 0)]);
        self.row(&[(CLK_WR_N, 1)]);
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            let rw = (self.byte() & 1) as u32;
            self.step(Step { rw, ..Step::IDLE });
        }
    }

    /// The end of a step: the edge, IE low, the write strobe.
    fn edge_then_idle(&mut self) {
        self.row(&[(IE, 0)]);
        self.row(&[(CLK, 0), (CLK_WR_N, 0)]);
        self.row(&[(CLK_WR_N, 1)]);
    }

    // ---- the MCU

    fn shift_out(&mut self, byte: u8) {
        for b in (0..8).rev() {
            self.row(&[(MOSI, (byte >> b) as u32 & 1)]);
            self.row(&[(SCK, 1)]);
            self.row(&[(SCK, 0)]);
        }
    }

    fn mcu_rx(&mut self, byte: u8) {
        self.shift_out(byte);
        self.row(&[(RCLK_RX, 1)]);
        self.row(&[(RCLK_RX, 0)]);
    }

    fn mcu_evt(&mut self, port: u8, data: u8) {
        self.shift_out(data);
        self.shift_out(port);
        self.row(&[(RCLK_EVT, 1)]);
        self.row(&[(RCLK_EVT, 0)]);
    }

    fn mcu_read_tx(&mut self) {
        self.row(&[(MBX_SEL, 0)]); // MISO shows bit 7
        for _ in 0..7 {
            self.row(&[(SCK, 1)]);
            self.row(&[(SCK, 0)]);
        }
        self.row(&[(MBX_SEL, 1)]); // TX full clears
    }
}

/// A CPU step's inputs; `port` / `data` None = random.
#[derive(Clone, Copy)]
struct Step {
    ie: u32,
    rw: u32,
    port: Option<u8>,
    data: Option<u8>,
    inr: u32,
    rst_n: u32,
}

impl Step {
    const IDLE: Step = Step { ie: 0, rw: 1, port: None, data: None, inr: 0, rst_n: 1 };

    fn io(rw: u32, port: u8) -> Step {
        Step { ie: 1, rw, port: Some(port), ..Step::IDLE }
    }
}

fn iod_sets() -> Vec<Vec<Row>> {
    let mut sets = Vec::new();

    // 1. every port, IN and OUT, IE on and off
    let mut k = Card::new(0x10D1_0001_5A5A_C3C3);
    k.mcu_rx(0x5A);
    k.mcu_evt(0xF3, 0xC4);
    for port in 0..=255u8 {
        for rw in [1, 0] {
            for ie in [1, 0] {
                k.step(Step { ie, ..Step::io(rw, port) });
                if k.m.tx_full {
                    k.mcu_read_tx(); // an OUT TX: the MCU reads it, TX clears
                }
                if !k.m.rx_full {
                    k.idle(2); // an IN RX emptied RX: write it again, later
                    let b = k.byte();
                    k.mcu_rx(b);
                }
            }
        }
    }
    sets.push(k.rows);

    // 2. the handshakes both ways; the 1 us rule and the race it avoids
    let mut k = Card::new(0x10D1_0002_0F0F_7E7E);
    for _ in 0..40 {
        let v = k.byte();
        let p = 0x01 + 4 * (k.byte() % 16);
        k.step(Step::io(1, p)); // STATUS: TX free
        let p = 4 * (k.byte() % 16);
        k.step(Step { data: Some(v), ..Step::io(0, p) }); // OUT TX
        k.step(Step::io(1, 0x01)); // STATUS: TX full
        k.mcu_read_tx(); // MISO = v, bit 7 first
        k.step(Step::io(1, 0x01)); // TX free again
        let w = k.byte();
        k.mcu_rx(w);
        k.step(Step::io(1, 0x01)); // RX available
        k.step(Step::io(1, 0x00)); // IN RX = w; RX still full in this step
        k.row(&[(CLK, 1)]); // next edge: rx_clr low, RX clears
        if k.byte() < 128 {
            let b = k.byte();
            k.mcu_rx(b); // RULE BROKEN: written during the clear, lost
            assert!(!k.m.rx_full);
        }
        k.edge_then_idle();
        k.idle(2); // >= 1 us later: allowed
        let b = k.byte();
        k.mcu_rx(b);
        assert!(k.m.rx_full);
        k.step(Step::io(1, 0x00));
        k.idle(2);
    }
    sets.push(k.rows);

    // 3. events: INT, the vector step, EVT cleared after it; one written during the clear is lost
    let mut k = Card::new(0x10D1_0003_1234_8765);
    for _ in 0..30 {
        let (port, data) = (k.byte(), k.byte());
        k.mcu_evt(port, data);
        let n = 1 + k.byte() as usize % 3;
        k.idle(n); // INT high from the next edge
        k.step(Step { inr: 1, ..Step::IDLE }); // the last push step
        k.step(Step::IDLE); // the vector step
        k.row(&[(CLK, 1)]); // step n + 2: EVT clears
        if k.byte() < 77 {
            let (p, d) = (k.byte(), k.byte());
            k.mcu_evt(p, d); // RULE BROKEN: lost
            assert!(!k.m.evt_pend);
        }
        k.edge_then_idle();
        k.idle(2);
    }
    sets.push(k.rows);

    // 4. reset in the middle: every flag set, then rst_n low; the latches keep their bytes
    let mut k = Card::new(0x10D1_0004_9999_0001);
    for _ in 0..10 {
        let b = k.byte();
        k.mcu_rx(b);
        let (p, d) = (k.byte(), k.byte());
        k.mcu_evt(p, d);
        k.step(Step::io(0, 0x00));
        k.idle(1);
        k.step(Step { rst_n: 0, ..Step::IDLE });
        k.step(Step { rst_n: 0, ..Step::IDLE });
        k.idle(1);
        k.step(Step::io(1, 0x01)); // STATUS: all clear
        assert_eq!(k.m.status(), hbc2_core::mailbox::TX_FREE);
    }
    sets.push(k.rows);
    sets
}

pub const MODULES: &[SeqModule] =
    &[SeqModule { name: "iod", inputs: IOD_IN, outputs: IOD_OUT, buses: IOD_BUSES, sets: iod_sets }];
