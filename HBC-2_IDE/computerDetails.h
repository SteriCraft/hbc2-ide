#ifndef COMPUTERDETAILS_H
#define COMPUTERDETAILS_H

/*!
 * \file computerDetails.h
 * \brief Constants, structures and enumerations according to the computer specifications.
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <string>
#include <QMutex>

using Byte = uint8_t;
using Word = uint16_t;
using Dword = uint32_t;

/*!
 * \namespace Cpu
 *
 * Central Processing Unit specifications.<br>
 * Find more info in the CPU documentation.
 */
namespace Cpu
{
    constexpr int IVT_START_ADDRESS = 0x0100; //!< IVT: Interrupt Vector Table

    constexpr int PROGRAM_START_ADDRESS = 0x0300; //!< More info in the CPU documentation

    constexpr int REGISTERS_NB = 8; //!< More info in the CPU documentation
    constexpr int FLAGS_NB = 8; //!< More info in the CPU documentation

    constexpr int INSTRUCTIONS_NB = 48; //!< More info in the CPU documentation
    constexpr int INSTRUCTION_SIZE = 4; //!< In bytes
    constexpr int ADDRESSING_MODES_NB = 8; //!< More info in the CPU documentation

    constexpr int OPCODE_MASK         = 0xFC000000; //!< <b>111111</b> 0000 000 000 00000000 00000000
    constexpr int ADDRMODE_MASK       = 0x03C00000; //!< 000000 <b>1111</b> 000 000 00000000 00000000
    constexpr int R1_MASK             = 0x00380000; //!< 000000 0000 <b>111</b> 000 00000000 00000000
    constexpr int R2_MASK             = 0x00070000; //!< 000000 0000 000 <b>111</b> 00000000 00000000
    constexpr int R3_MASK             = 0x00000700; //!< 000000 0000 000 000 00000<b>111</b> 00000000
    constexpr int V1_MASK             = 0x0000FF00; //!< 000000 0000 000 000 <b>11111111</b> 00000000
    constexpr int V2_MASK             = 0x000000FF; //!< 000000 0000 000 000 00000000 <b>11111111</b>
    constexpr int VX_MASK             = 0x0000FFFF; //!< 000000 0000 000 000 <b>11111111 11111111</b>
    constexpr int INTERRUPT_PORT_MASK     = 0x00FF; //!< 00000000 <b>11111111</b>

    /*!
     * \enum Register
     * \brief See Cpu::REGISTERS_NB
     */
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

    enum class CpuState { INSTRUCTION_EXEC = 0, INTERRUPT_MANAGEMENT = 1 };
}

/*!
 * \brief Used to send the current HbcCpu state to the CpuStateViewer
 */
struct CpuStatus
{
    Cpu::CpuState state = Cpu::CpuState::INSTRUCTION_EXEC;
    bool interruptReady = true;

    Word programCounter = Cpu::PROGRAM_START_ADDRESS;
    Dword instructionRegister = 0x0000;

    // == DECODED INSTRUCTION ==
    Cpu::InstructionOpcode opcode;
    Cpu::AddressingMode addrMode = Cpu::AddressingMode::NONE;

    Cpu::Register r1;
    Cpu::Register r2;
    Cpu::Register r3;

    Byte v1;
    Byte v2;
    Word vX;
    // =========================

    bool flags[Cpu::FLAGS_NB] = { false };
    Byte registers[Cpu::REGISTERS_NB] = { 0x00 };

    Byte stackPointer = 0x00;

    Word addressBus = 0x0000;
    Byte dataBus = 0x00;

    bool lastState = false;
};

/*!
 * \namespace Ram
 *
 * Random Access Memory specifications.<br>
 * Find more info in the CPU documentation.
 */
namespace Ram
{
    constexpr int MEMORY_SIZE = 0x10000; //!< 65,536 bytes
}

/*!
 * \namespace Iod
 *
 * Input/Output Device specifications.<br>
 * Find more info in the CPU documentation.
 */
namespace Iod
{
    constexpr int INTERRUPT_QUEUE_SIZE = 256;
    constexpr int PORTS_NB = 256;
    constexpr int IOD_RESERVED_PORT_NB = 0; //!< Used to HbcIod - HbcCpu data exchange

    /*!
     * \struct Interrupt
     * \brief Stores informations describing an interrupt (a software interrupt results in <i>data = 0x00</i>).
     */
    struct Interrupt
    {
        Byte portId;
        Byte data;
    };

    /*!
     * \struct PortSocket
     * \brief Used by peripherals to access Iod ports.
     */
    struct PortSocket
    {
        Byte portId; //!< Specified by HbcIod when ports are requested by peripherals
        Byte *portDataPointer; //!< Leads to Port.data
    };

    /*!
     * \struct Port
     * \brief Used by HbcIod to store port information, accessed by peripherals through PortSocket.
     */
    struct Port
    {
        Byte peripheralId; //!< HbcIod stores informations about connected peripherals <i>(#0 = no peripheral connected)</i>
        Byte data; //!< Where PortSocket.portDataPointer leads to
    };
}


#endif // COMPUTERDETAILS_H
