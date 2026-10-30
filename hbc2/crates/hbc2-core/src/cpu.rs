//! The HBC-2 CPU.
//!
//! One [`Cpu::tick`] is one clock cycle. A cycle either executes one whole
//! instruction, or performs one step of the interrupt handshake, or does
//! nothing (halted).
//!
//! Instructions with an addressing mode the manual does not list for them,
//! and unimplemented opcodes, behave as NOP.

use crate::board::Board;
use crate::isa::{
    AddrMode, Flag, INSTRUCTION_SIZE, IVT_START, Instruction, Opcode, PROGRAM_START, Register,
    STACK_RESET,
};

/// The 8-bit flag register.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags(pub u8);

impl Flags {
    #[inline(always)]
    pub fn get(self, flag: Flag) -> bool {
        self.0 & (1 << flag as u8) != 0
    }

    #[inline(always)]
    pub fn set(&mut self, flag: Flag, value: bool) {
        let mask = 1 << flag as u8;
        if value {
            self.0 |= mask;
        } else {
            self.0 &= !mask;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuState {
    /// Fetching and executing instructions.
    Executing,
    /// INR has been raised; waiting for the IOD to put the interrupt on the buses.
    InterruptAcknowledge,
}

#[derive(Clone, Debug)]
pub struct Cpu {
    pub registers: [u8; 8],
    pub flags: Flags,
    /// Address of the next instruction to execute.
    pub pc: u16,
    /// Stack pointer (STK): address of the last byte pushed. The stack grows
    /// downward from 0xFE00 (first push at 0xFDFF) and wraps around.
    pub sp: u16,
    /// Last fetched instruction.
    pub ir: Instruction,
    /// Address of the last executed instruction.
    pub last_pc: u16,
    pub state: CpuState,
    /// Software interrupt (port, data) raised by `INT`, waiting for the Interrupt flag.
    pending_software_interrupt: Option<(u8, u8)>,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu::new()
    }
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut cpu = Cpu {
            registers: [0; 8],
            flags: Flags::default(),
            pc: PROGRAM_START,
            sp: STACK_RESET,
            ir: Instruction(0),
            last_pc: PROGRAM_START,
            state: CpuState::Executing,
            pending_software_interrupt: None,
        };
        cpu.reset();
        cpu
    }

    /// Power-on state: registers cleared, interrupts enabled, PC at the program start.
    pub fn reset(&mut self) {
        self.registers = [0; 8];
        self.flags = Flags::default();
        self.flags.set(Flag::Interrupt, true);
        self.pc = PROGRAM_START;
        self.sp = STACK_RESET;
        self.ir = Instruction(0);
        self.last_pc = PROGRAM_START;
        self.state = CpuState::Executing;
        self.pending_software_interrupt = None;
    }

    #[inline(always)]
    pub fn reg(&self, r: Register) -> u8 {
        self.registers[r as usize]
    }

    #[inline(always)]
    pub fn set_reg(&mut self, r: Register, value: u8) {
        self.registers[r as usize] = value;
    }

    #[inline(always)]
    pub fn flag(&self, flag: Flag) -> bool {
        self.flags.get(flag)
    }

    pub fn is_halted(&self) -> bool {
        self.flags.get(Flag::Halt)
    }

    pub fn pending_software_interrupt(&self) -> Option<(u8, u8)> {
        self.pending_software_interrupt
    }

    /// One clock cycle. Returns true when the PC moved to a new instruction
    /// (an instruction was executed, or an interrupt jumped to its handler).
    #[inline(always)]
    pub fn tick(&mut self, board: &mut Board) -> bool {
        match self.state {
            CpuState::InterruptAcknowledge => {
                // Step 4: the IOD lowered INT and put port + data on the buses.
                if board.int {
                    return false;
                }
                board.inr = false;
                let port = board.address_bus as u8;
                let data = board.data_bus;
                self.enter_interrupt(board, port, data);
                true
            }
            CpuState::Executing => {
                if self.flags.get(Flag::Interrupt) {
                    if board.int {
                        // Step 2: hardware interrupt pending, signal we are ready.
                        board.inr = true;
                        self.state = CpuState::InterruptAcknowledge;
                        return false;
                    }
                    // Software interrupts have no priority over hardware ones.
                    if let Some((port, data)) = self.pending_software_interrupt.take() {
                        board.address_bus = port as u16;
                        board.data_bus = data;
                        self.enter_interrupt(board, port, data);
                        return true;
                    }
                }
                if self.flags.get(Flag::Halt) {
                    return false;
                }
                self.execute_next(board);
                true
            }
        }
    }

    /// Pushes I and PC, loads the data into I, disables interrupts and jumps
    /// to the handler found in the interrupt vector table.
    fn enter_interrupt(&mut self, board: &mut Board, port: u8, data: u8) {
        self.push(board, self.reg(Register::I));
        self.push_address(board, self.pc);
        self.set_reg(Register::I, data);
        self.flags.set(Flag::Interrupt, false);
        self.flags.set(Flag::Halt, false);
        self.pc = board.ram.read_u16(IVT_START + port as u16 * 2);
        self.state = CpuState::Executing;
    }

    // ----- Stack (grows downward). PSH: STK = STK - 1, $(STK) = R1. POP: R1 = $(STK), STK = STK + 1.

    #[inline(always)]
    fn push(&mut self, board: &mut Board, value: u8) {
        self.sp = self.sp.wrapping_sub(1);
        board.ram.write(self.sp, value);
    }

    #[inline(always)]
    fn pop(&mut self, board: &mut Board) -> u8 {
        let value = board.ram.read(self.sp);
        self.sp = self.sp.wrapping_add(1);
        value
    }

    /// Pushes LSB then MSB, so that RET / IRT pop the MSB first (as the manual describes).
    #[inline(always)]
    fn push_address(&mut self, board: &mut Board, address: u16) {
        self.push(board, address as u8);
        self.push(board, (address >> 8) as u8);
    }

    #[inline(always)]
    fn pop_address(&mut self, board: &mut Board) -> u16 {
        let msb = self.pop(board);
        let lsb = self.pop(board);
        u16::from_be_bytes([msb, lsb])
    }

    // ----- Flags helpers

    #[inline(always)]
    fn set_zn(&mut self, value: u8) {
        self.flags.set(Flag::Zero, value == 0);
        self.flags.set(Flag::Negative, value & 0x80 != 0);
    }

    #[inline(always)]
    fn set_czn(&mut self, carry: bool, value: u8) {
        self.flags.set(Flag::Carry, carry);
        self.set_zn(value);
    }

    #[inline(always)]
    fn r(&self, index: u8) -> u8 {
        self.registers[(index & 7) as usize]
    }

    #[inline(always)]
    fn set_r(&mut self, index: u8, value: u8) {
        self.registers[(index & 7) as usize] = value;
    }

    /// R1:R2 as a 16-bit value (R1 = MSB).
    #[inline(always)]
    fn r1r2(&self, i: Instruction) -> u16 {
        u16::from_be_bytes([self.r(i.r1()), self.r(i.r2())])
    }

    /// RamReg / ImmReg / Offset address: (R1:R2) + V2, wrapping in 16 bits.
    #[inline(always)]
    fn pointer_address(&self, i: Instruction) -> u16 {
        self.r1r2(i).wrapping_add(i.v2() as u16)
    }

    /// Stack / Offset address: STK + V1, wrapping in 16 bits.
    #[inline(always)]
    fn stack_address(&self, i: Instruction) -> u16 {
        self.sp.wrapping_add(i.v1() as u16)
    }

    /// Operands of the ALU instructions and CMP: (index of the data register,
    /// second operand). The data register is R3 in RamReg / ImmReg / Offset,
    /// R1 otherwise.
    #[inline(always)]
    fn alu_operands(&self, board: &Board, mode: AddrMode, i: Instruction) -> Option<(u8, u8)> {
        match mode {
            AddrMode::Reg => Some((i.r1(), self.r(i.r2()))),
            AddrMode::RegImm8 => Some((i.r1(), i.v1())),
            AddrMode::RegRam => Some((i.r1(), board.ram.read(i.vx()))),
            AddrMode::RamRegImmReg => Some((i.r3(), board.ram.read(self.pointer_address(i)))),
            AddrMode::StackOffset => Some((i.r1(), board.ram.read(self.stack_address(i)))),
            _ => None,
        }
    }

    /// Target address of jumps and CAL (Reg16 or Imm16).
    #[inline(always)]
    fn jump_target(&self, mode: AddrMode, i: Instruction) -> Option<u16> {
        match mode {
            AddrMode::Reg16 => Some(self.r1r2(i)),
            AddrMode::Imm16 => Some(i.vx()),
            _ => None,
        }
    }

    /// Fetch, decode and execute the instruction at PC.
    #[inline(always)]
    fn execute_next(&mut self, board: &mut Board) {
        let address = self.pc;
        let i = Instruction(board.ram.read_u32(address));
        self.ir = i;
        self.last_pc = address;
        self.pc = address.wrapping_add(INSTRUCTION_SIZE);

        let (Some(op), Some(mode)) = (i.opcode(), i.mode()) else {
            return;
        };
        self.execute(board, op, mode, i);
    }

    #[inline(always)]
    fn execute(&mut self, board: &mut Board, op: Opcode, mode: AddrMode, i: Instruction) {
        use AddrMode as M;
        use Opcode as O;

        match op {
            O::Nop => {}

            // ----- Arithmetic and logic: R1 = R1 <op> operand
            O::Adc | O::Add => {
                if let Some((dest, b)) = self.alu_operands(board, mode, i) {
                    let carry_in = (op == O::Adc && self.flags.get(Flag::Carry)) as u16;
                    let sum = self.r(dest) as u16 + b as u16 + carry_in;
                    let result = sum as u8;
                    self.set_czn(sum > 0xFF, result);
                    self.set_r(dest, result);
                }
            }
            O::Sub | O::Sbb => {
                if let Some((dest, b)) = self.alu_operands(board, mode, i) {
                    let borrow_in = (op == O::Sbb && self.flags.get(Flag::Carry)) as i16;
                    let diff = self.r(dest) as i16 - b as i16 - borrow_in;
                    let result = diff as u8;
                    self.set_czn(diff < 0, result);
                    self.set_r(dest, result);
                }
            }
            O::And | O::Or | O::Xor => {
                if let Some((dest, b)) = self.alu_operands(board, mode, i) {
                    let a = self.r(dest);
                    let result = match op {
                        O::And => a & b,
                        O::Or => a | b,
                        _ => a ^ b,
                    };
                    self.set_zn(result);
                    self.set_r(dest, result);
                }
            }
            O::Cmp => {
                // CMP has no Reg / Ram mode.
                let pair = match mode {
                    M::RegRam => None,
                    _ => self
                        .alu_operands(board, mode, i)
                        .map(|(dest, b)| (self.r(dest), b)),
                };
                if let Some((a, b)) = pair {
                    self.flags.set(Flag::Superior, a > b);
                    self.flags.set(Flag::Equal, a == b);
                    self.flags.set(Flag::Inferior, a < b);
                }
            }
            O::Inc | O::Dec => {
                let increment = op == O::Inc;
                let apply = |cpu: &mut Cpu, value: u8| -> u8 {
                    let (result, carry) = if increment {
                        value.overflowing_add(1)
                    } else {
                        value.overflowing_sub(1)
                    };
                    cpu.set_czn(carry, result);
                    result
                };
                match mode {
                    M::Reg => {
                        let value = self.r(i.r1());
                        let result = apply(self, value);
                        self.set_r(i.r1(), result);
                    }
                    M::Imm16 | M::RamRegImmReg => {
                        let address = if mode == M::Imm16 {
                            i.vx()
                        } else {
                            self.pointer_address(i)
                        };
                        let value = board.ram.read(address);
                        let result = apply(self, value);
                        board.ram.write(address, result);
                    }
                    _ => {}
                }
            }
            O::Not => match mode {
                M::Reg => {
                    let result = !self.r(i.r1());
                    self.set_zn(result);
                    self.set_r(i.r1(), result);
                }
                M::Imm16 => {
                    let result = !board.ram.read(i.vx());
                    self.set_zn(result);
                    board.ram.write(i.vx(), result);
                }
                _ => {}
            },
            O::Shl | O::Asr | O::Shr => {
                if mode == M::Reg {
                    let value = self.r(i.r1());
                    let result = match op {
                        O::Shl => {
                            self.flags.set(Flag::Carry, value & 0x80 != 0);
                            value << 1
                        }
                        O::Asr => {
                            self.flags.set(Flag::Carry, value & 0x01 != 0);
                            (value >> 1) | (value & 0x80)
                        }
                        // SHR discards the shifted bit: the carry flag is not affected.
                        _ => value >> 1,
                    };
                    self.set_zn(result);
                    self.set_r(i.r1(), result);
                }
            }

            // ----- Memory and registers
            O::Mov => match mode {
                M::Reg => self.set_r(i.r1(), self.r(i.r2())),
                M::RegImm8 => self.set_r(i.r1(), i.v1()),
                _ => {}
            },
            O::Lod => match mode {
                M::RegRam => self.set_r(i.r1(), board.ram.read(i.vx())),
                M::RamRegImmReg => self.set_r(i.r3(), board.ram.read(self.pointer_address(i))),
                M::StackOffset => self.set_r(i.r1(), board.ram.read(self.stack_address(i))),
                _ => {}
            },
            O::Str => match mode {
                M::RegRam => board.ram.write(i.vx(), self.r(i.r1())),
                M::RamRegImmReg => board.ram.write(self.pointer_address(i), self.r(i.r3())),
                M::StackOffset => board.ram.write(self.stack_address(i), self.r(i.r1())),
                _ => {}
            },
            O::Psh => {
                if mode == M::Reg {
                    self.push(board, self.r(i.r1()));
                }
            }
            O::Pop => {
                if mode == M::Reg {
                    let value = self.pop(board);
                    self.set_r(i.r1(), value);
                }
            }
            O::Psf => {
                if mode == M::None {
                    self.push(board, self.flags.0);
                }
            }
            // All 8 flags are restored, Interrupt and Halt included.
            O::Pof => {
                if mode == M::None {
                    self.flags = Flags(self.pop(board));
                }
            }

            // ----- Stack pointer (no flags affected)
            O::Asp | O::Ssp => {
                let amount = match mode {
                    M::Imm8 => Some(i.v1() as u16),
                    M::Imm16 => Some(i.vx()),
                    _ => None,
                };
                if let Some(amount) = amount {
                    self.sp = if op == O::Asp {
                        self.sp.wrapping_add(amount)
                    } else {
                        self.sp.wrapping_sub(amount)
                    };
                }
            }
            O::Rsp => {
                if mode == M::Reg16 {
                    let [msb, lsb] = self.sp.to_be_bytes();
                    self.set_r(i.r1(), msb);
                    self.set_r(i.r2(), lsb);
                }
            }
            O::Wsp => match mode {
                M::Reg16 => self.sp = self.r1r2(i),
                M::Imm16 => self.sp = i.vx(),
                _ => {}
            },

            // ----- Control flow
            O::Jmp | O::Jmc | O::Jme | O::Jmn | O::Jms | O::Jmz | O::Jmf => {
                let condition = match op {
                    O::Jmp => true,
                    O::Jmc => self.flags.get(Flag::Carry),
                    O::Jme => self.flags.get(Flag::Equal),
                    O::Jmn => self.flags.get(Flag::Negative),
                    O::Jms => self.flags.get(Flag::Superior),
                    O::Jmz => self.flags.get(Flag::Zero),
                    _ => self.flags.get(Flag::Inferior),
                };
                if condition && let Some(target) = self.jump_target(mode, i) {
                    self.pc = target;
                }
            }
            O::Cal => {
                if let Some(target) = self.jump_target(mode, i) {
                    // Return address = the instruction after CAL.
                    self.push_address(board, self.pc);
                    self.pc = target;
                }
            }
            O::Ret => {
                if mode == M::None {
                    self.pc = self.pop_address(board);
                }
            }
            O::Irt => {
                if mode == M::None {
                    self.pc = self.pop_address(board);
                    let i_register = self.pop(board);
                    self.set_reg(Register::I, i_register);
                    self.flags.set(Flag::Interrupt, true);
                }
            }
            O::Int => {
                if mode == M::Imm8 {
                    let data = self.reg(Register::I);
                    board.address_bus = i.v1() as u16;
                    board.data_bus = data;
                    self.pending_software_interrupt = Some((i.v1(), data));
                }
            }
            O::Hlt => {
                if mode == M::None {
                    self.flags.set(Flag::Halt, true);
                    self.flags.set(Flag::Interrupt, true);
                }
            }

            // ----- I/O
            O::In => {
                if mode == M::Reg {
                    let port = self.r(i.r2());
                    let value = board.iod.port(port);
                    board.address_bus = port as u16;
                    board.data_bus = value;
                    self.set_r(i.r1(), value);
                }
            }
            O::Out => {
                if mode == M::Reg {
                    let port = self.r(i.r1());
                    let value = self.r(i.r2());
                    board.address_bus = port as u16;
                    board.data_bus = value;
                    board.iod.cpu_write(port, value);
                }
            }

            // ----- Flag set / clear
            O::Clc => self.set_flag_if_none(mode, Flag::Carry, false),
            O::Cle => self.set_flag_if_none(mode, Flag::Equal, false),
            O::Cli => self.set_flag_if_none(mode, Flag::Interrupt, false),
            O::Cln => self.set_flag_if_none(mode, Flag::Negative, false),
            O::Cls => self.set_flag_if_none(mode, Flag::Superior, false),
            O::Clz => self.set_flag_if_none(mode, Flag::Zero, false),
            O::Clf => self.set_flag_if_none(mode, Flag::Inferior, false),
            O::Stc => self.set_flag_if_none(mode, Flag::Carry, true),
            O::Ste => self.set_flag_if_none(mode, Flag::Equal, true),
            O::Sti => self.set_flag_if_none(mode, Flag::Interrupt, true),
            O::Stn => self.set_flag_if_none(mode, Flag::Negative, true),
            O::Sts => self.set_flag_if_none(mode, Flag::Superior, true),
            O::Stz => self.set_flag_if_none(mode, Flag::Zero, true),
            O::Stf => self.set_flag_if_none(mode, Flag::Inferior, true),
        }
    }

    #[inline(always)]
    fn set_flag_if_none(&mut self, mode: AddrMode, flag: Flag, value: bool) {
        if mode == AddrMode::None {
            self.flags.set(flag, value);
        }
    }
}
