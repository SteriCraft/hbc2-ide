#ifndef COMPUTERDETAILS_H
#define COMPUTERDETAILS_H

#include <string>

#define PORTS_NB 256

#define ADDRESS_NOT_SET 0x0000
#define PROGRAM_START_ADDRESS 0x0300

#define MEMORY_SIZE 0x10000 // 65'536 bytes

#define REGISTERS_NB 8
#define FLAGS_NB 8

#define INSTRUCTIONS_NB 48
#define INSTRUCTION_SIZE 4 // In bytes
#define ADDRESSING_MODES_NB 8

#define OPCODE_MASK         0xFC000000
#define ADDRMODE_MASK       0x03C00000
#define R1_MASK             0x00380000
#define R2_MASK             0x00070000
#define R3_MASK             0x00000700
#define V1_MASK             0x0000FF00
#define V2_MASK             0x000000FF
#define VX_MASK             0x0000FFFF
#define INTERRUPT_PORT_MASK     0x00FF

#define IVT_START_ADDRESS 0x0100

using Byte = uint8_t;
using Word = uint16_t;
using Dword = uint32_t;

namespace Computer
{
enum class Register { A = 0, B = 1, C = 2, D = 3, I = 4, J = 5, X = 6, Y = 7 };
const std::string regStrArr[] = { "a", "b", "c", "d", "i", "j", "x", "y" };

enum class Flags { CARRY = 0, EQUAL = 1, INTERRUPT = 2, NEGATIVE = 3, SUPERIOR = 4, ZERO = 5, INFERIOR = 6, HALT = 7 };

enum class InstructionOpcode { NOP = 0,  ADC = 1,  ADD = 2,  AND = 3,  CAL = 4,  CLC = 5,  CLE = 6,  CLI = 7,  CLN = 8,
                               CLS = 9,  CLZ = 10, CLF = 11, CMP = 12, DEC = 13, HLT = 14, IN  = 15, OUT = 16, INC = 17,
                               INT = 18, IRT = 19, JMC = 20, JME = 21, JMN = 22, JMP = 23, JMS = 24, JMZ = 25, JMF = 26,
                               STR = 27, LOD = 28, MOV = 29, NOT = 30, OR  = 31, POP = 32, PSH = 33, RET = 34, SHL = 35,
                               ASR = 36, SHR = 37, STC = 38, STE = 39, STI = 40, STN = 41, STS = 42, STZ = 43, STF = 44,
                               SUB = 45, SBB = 46, XOR = 47 };
const std::string instrStrArr[] = { "nop", "adc", "add", "and", "cal", "clc", "cle", "cli", "cln",
                                   "cls", "clz", "clf", "cmp", "dec", "hlt", "in",  "out", "inc",
                                   "int", "irt", "jmc", "jme", "jmn", "jmp", "jms", "jmz", "jmf",
                                   "str", "lod", "mov", "not", "or",  "pop", "psh", "ret", "shl",
                                   "asr", "shr", "stc", "ste", "sti", "stn", "sts", "stz", "stf",
                                   "sub", "sbb", "xor" };

enum class AddressingMode { NONE = 0, REG = 1, REG_IMM8 = 2, REG_RAM = 3,
                            RAMREG_IMMREG = 4, REG16 = 5, IMM16 = 6, IMM8 = 7 };

enum class CpuState { INSTRUCTION_EXEC = 0, INTERRUPT_MANAGEMENT };
}

#endif // COMPUTERDETAILS_H
