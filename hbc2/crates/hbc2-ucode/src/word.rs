//! The microcode word: 54 bits in 7 AT28C256 (56 bits, 2 spare), and the
//! EEPROM address (cu-decisions.md sections 1-3).

/// One field of the microcode word: first bit, width, name (the signal it
/// drives, or the encoded field of cu-decisions.md section 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub bit: u8,
    pub width: u8,
    pub name: &'static str,
    /// Value when the step doesn't use it (nothing driven, nothing loaded).
    pub idle: u8,
}

macro_rules! fields {
    ($($id:ident = $bit:expr, $w:expr, $name:expr, $idle:expr;)*) => {
        $(pub const $id: Field = Field { bit: $bit, width: $w, name: $name, idle: $idle };)*
        /// Every field, in bit order.
        pub const FIELDS: &[Field] = &[$($id),*];
    };
}

// EEPROM k holds bits 8k..8k+7. The order groups each section's controls on
// as few EEPROMs as possible.
fields! {
    ALU_OP          = 0, 4, "alu_op", 0b0000;
    FLAGS_OP        = 4, 4, "flags_op", 0b0000;
    FLAGS_BIT_VAL   = 8, 1, "flags_bit_val", 0;
    REGS_WE_N       = 9, 1, "regs_we_n", 1;
    REGS_A_SEL      = 10, 2, "regs_a_sel", 0b11;
    REGS_CU_IDX     = 12, 2, "regs_cu_idx", 0b00;
    REGS_CU_BANK_N  = 14, 2, "regs_cu_bank_n", 0b11;
    REGS_B_SEL      = 16, 2, "regs_b_sel", 0b11;
    REGS_CONST_OE_N = 18, 1, "regs_const_oe_n", 1;
    DBUS            = 19, 3, "dbus", 0;
    ABUS            = 22, 2, "abus", 0b00;
    ADDR_BASE_SEL   = 24, 2, "addr_base_sel", 0b00;
    ADDR_BASE_OE_N  = 26, 2, "addr_base_oe_n", 0b11;
    ADDR_OFF_SEL    = 28, 1, "addr_off_sel", 0;
    ADDR_OFF_OE_N   = 29, 2, "addr_off_oe_n", 0b11;
    ADDR_INV        = 31, 1, "addr_inv", 0;
    ADDR_CIN        = 32, 1, "addr_cin", 0;
    ADDR_LAT_LD_N   = 33, 1, "addr_lat_ld_n", 1;
    ADDR_PC_LD_N    = 34, 1, "addr_pc_ld_n", 1;
    ADDR_SP_LD_N    = 35, 1, "addr_sp_ld_n", 1;
    IR_B0_LD_N      = 36, 1, "ir_b0_ld_n", 1;
    IR_B1_LD_N      = 37, 1, "ir_b1_ld_n", 1;
    IR_V1_LD_N      = 38, 1, "ir_v1_ld_n", 1;
    IR_V2_LD_N      = 39, 1, "ir_v2_ld_n", 1;
    MEM_RE          = 40, 1, "mem_re", 0;
    MEM_OE_N        = 41, 1, "mem_oe_n", 1;
    MEM_WE_N        = 42, 1, "mem_we_n", 1;
    MEM_WBUF_OE_N   = 43, 1, "mem_wbuf_oe_n", 1;
    MEM_ROM_OE_N    = 44, 1, "mem_rom_oe_n", 1;
    INR             = 45, 1, "inr", 0;
    CU_SWI_LD_N     = 46, 1, "cu_swi_ld_n", 1;
    CU_FORCE        = 47, 1, "cu_force", 0;
    CU_NEXT         = 48, 4, "cu_next", 0;
    CU_COND_SEL     = 52, 2, "cu_cond_sel", 0b00;
    SPARE           = 54, 2, "spare", 0b11;
}

/// Bits used by the microcode (the spare pair excluded).
pub const USED_BITS: u32 = 54;
/// AT28C256s holding the microcode.
pub const ROMS: usize = 7;
/// Words in each EEPROM: the 15-bit address.
pub const WORDS: usize = 1 << 15;

/// `dbus` codes: the CPU's driver on `bus_data` (74HCT138).
pub mod dbus {
    pub const NONE: u8 = 0;
    pub const BUS_A: u8 = 1;
    pub const RES: u8 = 2;
    pub const FLAGS: u8 = 3;
    pub const PC_HI: u8 = 4;
    pub const PC_LO: u8 = 5;
    pub const SP_HI: u8 = 6;
    pub const SP_LO: u8 = 7;
}

/// `abus` codes: who drives `bus_addr`.
pub mod abus {
    /// The address latch.
    pub const LATCH: u8 = 0b00;
    /// The latch, with IE = 1 (IN, OUT).
    pub const IO: u8 = 0b01;
    /// Hardware vector step: latch off, the IOD drives 00:p.
    pub const HW_VECTOR: u8 = 0b10;
    /// Software vector step: latch high byte (00), pending latch low byte.
    pub const SW_VECTOR: u8 = 0b11;
}

/// `cu_cond_sel` codes (74HCT153 half): the input on address bit A0.
pub mod cond {
    /// The jump condition (74HCT151 by opcode bits 2..0).
    pub const JC: u8 = 0b00;
    /// `bnd_n`: no hardware or software interrupt taken, not halted.
    pub const NOINT: u8 = 0b01;
    /// `hw_q`: hardware interrupt, sampled at the end of the step before.
    pub const HWQ: u8 = 0b10;
    /// `x_n`: no software interrupt, no `cout_q`, not in reset.
    pub const XN: u8 = 0b11;
}

/// A 56-bit microcode word (bits 54-55 spare, kept at 1 as erased).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Word(pub u64);

impl Word {
    /// Every field at its idle value.
    pub fn idle() -> Word {
        let mut w = Word(0);
        for f in FIELDS {
            w.set(*f, f.idle);
        }
        w
    }

    #[inline(always)]
    pub fn get(self, f: Field) -> u8 {
        ((self.0 >> f.bit) & ((1 << f.width) - 1)) as u8
    }

    #[inline(always)]
    pub fn bit(self, f: Field) -> bool {
        self.get(f) != 0
    }

    pub fn set(&mut self, f: Field, value: u8) {
        let mask = ((1u64 << f.width) - 1) << f.bit;
        assert!(u64::from(value) < 1 << f.width, "{} = {value}", f.name);
        self.0 = (self.0 & !mask) | (u64::from(value) << f.bit);
    }

    /// The word without its sequencing fields (next, force, cond).
    pub fn controls(self) -> u64 {
        let seq = (0xFu64 << CU_NEXT.bit) | (1 << CU_FORCE.bit) | (3 << CU_COND_SEL.bit);
        self.0 & !seq
    }

    /// The fields that differ from idle, as "name=value".
    pub fn describe(self) -> String {
        self.describe_fields(|_| true)
    }

    /// As [`Word::describe`], without the sequencing fields.
    pub fn describe_controls(self) -> String {
        self.describe_fields(|f| f.bit < CU_NEXT.bit && f != CU_FORCE)
    }

    fn describe_fields(self, keep: impl Fn(Field) -> bool) -> String {
        let idle = Word::idle();
        FIELDS
            .iter()
            .filter(|f| keep(**f) && self.get(**f) != idle.get(**f))
            .map(|f| {
                if f.width == 1 {
                    format!("{}={}", f.name, self.get(*f))
                } else {
                    format!("{}={:0w$b}", f.name, self.get(*f), w = f.width as usize)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl std::fmt::Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Word({:014X}: {})", self.0, self.describe())
    }
}

/// EEPROM address: A14-A9 opcode, A8-A5 mode, A4-A1 step, A0 cond.
#[inline(always)]
pub const fn address(opcode: u8, mode: u8, step: u8, cond: bool) -> usize {
    ((opcode as usize & 0x3F) << 9)
        | ((mode as usize & 0xF) << 5)
        | ((step as usize & 0xF) << 1)
        | cond as usize
}

/// The 7 EEPROM images (32 KiB each) of a full microcode.
pub fn images(words: &[Word]) -> Vec<Vec<u8>> {
    assert_eq!(words.len(), WORDS);
    (0..ROMS)
        .map(|k| words.iter().map(|w| (w.0 >> (8 * k)) as u8).collect())
        .collect()
}

/// Rebuilds the words from the 7 images (what the hardware reads).
pub fn from_images(images: &[Vec<u8>]) -> Vec<Word> {
    assert_eq!(images.len(), ROMS);
    (0..WORDS)
        .map(|a| {
            let mut v = 0u64;
            for (k, image) in images.iter().enumerate() {
                v |= u64::from(image[a]) << (8 * k);
            }
            Word(v)
        })
        .collect()
}

/// The control outputs of the pipeline register for a word: the ROM bits
/// and the signals decoded before the 574s (cu-decisions.md section 2), named
/// as their receiving section names them, (name, width, value). The
/// sequencing fields that stay inside the control unit (`cu_next`,
/// `cu_force`, `cu_cond_sel`, `cu_swi_ld_n`, `cu_swi_oe_n`) are not listed.
pub fn outputs(w: Word) -> Vec<(&'static str, u32, u32)> {
    let g = |f: Field| u32::from(w.get(f));
    let d = g(DBUS);
    let not_dbus = |k: u8| u32::from(d != u32::from(k)); // the 74HCT138's active-low outputs
    let ab = w.get(ABUS);
    let (lat_hi, lat_lo, ie) = match ab {
        abus::LATCH => (true, true, false),
        abus::IO => (true, true, true),
        abus::HW_VECTOR => (false, false, false),
        _ => (true, false, false),
    };
    let lat_oe_n = (u32::from(!lat_hi) << 1) | u32::from(!lat_lo);
    let const_on = g(REGS_CONST_OE_N) == 0;
    vec![
        ("alu_op", 4, g(ALU_OP)),
        ("flags_op", 4, g(FLAGS_OP)),
        ("flags_bit_val", 1, g(FLAGS_BIT_VAL)),
        ("flags_oe_n", 1, not_dbus(dbus::FLAGS)),
        ("regs_we_n", 1, g(REGS_WE_N)),
        ("regs_a_sel", 2, g(REGS_A_SEL)),
        ("regs_cu_idx", 2, g(REGS_CU_IDX)),
        ("regs_cu_bank_n", 2, g(REGS_CU_BANK_N)),
        ("regs_b_sel", 2, g(REGS_B_SEL)),
        ("regs_a_lat_oe_n", 1, u32::from(g(REGS_CU_BANK_N) != 0b11)),
        (
            "regs_b_lat_oe_n",
            1,
            u32::from(!(g(REGS_B_SEL) == 0b11 && !const_on)),
        ),
        ("regs_const_oe_n", 1, g(REGS_CONST_OE_N)),
        ("regs_const_0", 1, u32::from(g(ALU_OP) & 0b1000 == 0)),
        ("regs_a_bus_oe_n", 1, not_dbus(dbus::BUS_A)),
        ("regs_res_oe_n", 1, not_dbus(dbus::RES)),
        ("addr_base_sel", 2, g(ADDR_BASE_SEL)),
        ("addr_base_oe_n", 2, g(ADDR_BASE_OE_N)),
        ("addr_off_sel", 1, g(ADDR_OFF_SEL)),
        ("addr_off_oe_n", 2, g(ADDR_OFF_OE_N)),
        ("addr_inv", 1, g(ADDR_INV)),
        ("addr_cin", 1, g(ADDR_CIN)),
        ("addr_lat_ld_n", 1, g(ADDR_LAT_LD_N)),
        ("addr_pc_ld_n", 1, g(ADDR_PC_LD_N)),
        ("addr_sp_ld_n", 1, g(ADDR_SP_LD_N)),
        ("addr_lat_oe_n", 2, lat_oe_n),
        (
            "addr_pc_oe_n",
            2,
            (not_dbus(dbus::PC_HI) << 1) | not_dbus(dbus::PC_LO),
        ),
        (
            "addr_sp_oe_n",
            2,
            (not_dbus(dbus::SP_HI) << 1) | not_dbus(dbus::SP_LO),
        ),
        ("ir_b0_ld_n", 1, g(IR_B0_LD_N)),
        ("ir_b1_ld_n", 1, g(IR_B1_LD_N)),
        ("ir_v1_ld_n", 1, g(IR_V1_LD_N)),
        ("ir_v2_ld_n", 1, g(IR_V2_LD_N)),
        ("mem_re", 1, g(MEM_RE)),
        ("mem_oe_n", 1, g(MEM_OE_N)),
        ("mem_we_n", 1, g(MEM_WE_N)),
        ("mem_wbuf_oe_n", 1, g(MEM_WBUF_OE_N)),
        ("mem_rom_oe_n", 1, g(MEM_ROM_OE_N)),
        ("ie", 1, u32::from(ie)),
        ("rw", 1, u32::from(d != u32::from(dbus::BUS_A))),
        ("inr", 1, g(INR)),
    ]
}

/// The pending port latch drives `bus_addr` low (`cu_swi_oe_n` = 0).
pub fn swi_oe(w: Word) -> bool {
    w.get(ABUS) == abus::SW_VECTOR
}

/// What the pull resistors hold the pipeline outputs at while the 574s are
/// off during reset (control-requirements.md C24): the idle word, with the
/// sequencer's lines at the reset word's values (`cu_cond_sel` = 11 `xn`,
/// `cu_force` = 0), so the address stays on the reset word.
pub fn pulled() -> Word {
    let mut w = Word::idle();
    w.set(CU_COND_SEL, cond::XN);
    w.set(CU_FORCE, 0);
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_tile_56_bits() {
        let mut next = 0;
        for f in FIELDS {
            assert_eq!(f.bit, next, "{}", f.name);
            next += f.width;
        }
        assert_eq!(next, 56);
        assert_eq!(SPARE.bit as u32, USED_BITS);
    }
}
