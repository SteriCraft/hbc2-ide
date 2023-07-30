#ifndef CPU_H
#define CPU_H

#include <QString>
#include <QDebug>

namespace CPU
{
    #define CPU_START_ADDRESS 0x0300

    #define CPU_REGISTER_NB 8
    enum class Register { A = 0, B = 1, C = 2, D = 3, I = 4, J = 5, X = 6, Y = 7 };

    #define FLAGS_NB 8
    enum class Flags { CARRY = 0, EQUAL = 1, INTERRUPT = 2, NEGATIVE = 3, SUPERIOR = 4, ZERO = 5, INFERIOR = 6, HALT = 7 };

    #define OPCODE_MASK     0xFC000000
    #define ADDRMODE_MASK   0x03C00000
    #define R1_MASK         0x00380000
    #define R2_MASK         0x00070000
    #define R3_MASK         0x00000700
    #define V1_MASK         0x0000FF00
    #define V2_MASK         0x000000FF
    #define VX_MASK         0x0000FFFF

    #define INSTR_NB 48
    #define INSTRUCTION_SIZE 4 // In bytes
    enum class InstrOpcode { NOP = 0,  ADC = 1,  ADD = 2,  AND = 3,  CAL = 4,  CLC = 5,  CLE = 6,  CLI = 7,  CLN = 8,
                             CLS = 9,  CLZ = 10, CLF = 11, CMP = 12, DEC = 13, HLT = 14, IN  = 15, OUT = 16, INC = 17,
                             INT = 18, IRT = 19, JMC = 20, JME = 21, JMN = 22, JMP = 23, JMS = 24, JMZ = 25, JMF = 26,
                             STR = 27, LOD = 28, MOV = 29, NOT = 30, OR  = 31, POP = 32, PSH = 33, RET = 34, SHL = 35,
                             ASR = 36, SHR = 37, STC = 38, STE = 39, STI = 40, STN = 41, STS = 42, STZ = 43, STF = 44,
                             SUB = 45, SBB = 46, XOR = 47};
    const std::string instrStrArr[INSTR_NB] = { "nop", "adc", "add", "and", "cal", "clc", "cle", "cli", "cln",
                                                "cls", "clz", "clf", "cmp", "dec", "hlt", "in",  "out", "inc",
                                                "int", "irt", "jmc", "jme", "jmn", "jmp", "jms", "jmz", "jmf",
                                                "str", "lod", "mov", "not", "or",  "pop", "psh", "ret", "shl",
                                                "asr", "shr", "stc", "ste", "sti", "stn", "sts", "stz", "stf",
                                                "sub", "sbb", "xor"};

    #define ADDR_MODE_NB 8
    enum class AddrMode { NONE = 0, REG = 1, REG_IMM8 = 2, REG_RAM = 3,
                          RAMREG_IMMREG = 4, REG16 = 5, IMM16 = 6, IMM8 = 7 };
}

class HbcMotherboard;

// Code
class HbcCpu // SINGLETON
{
    static HbcCpu *m_singleton;

public:
    static HbcCpu* getInstance(HbcMotherboard *motherboard);

    void init();
    void tick(); // Executes one instruction completely

private:
    HbcCpu(HbcMotherboard *motherboard);

    void fetch();
    void execute();

    void jump();
    bool pop(uint8_t &data); // Returns true if stack underflows
    bool push(uint8_t data); // Returns true if stack overflows

    uint8_t m_registers[CPU_REGISTER_NB];
    bool m_flags[FLAGS_NB];
    uint32_t m_instructionRegister;

    bool m_jumpOccured;
    uint16_t m_programCounter;
    uint8_t m_stackPointer;
    bool m_stackIsFull;

    CPU::InstrOpcode m_opcode;
    CPU::AddrMode m_addressingMode;

    CPU::Register m_register1Index;
    CPU::Register m_register2Index;
    CPU::Register m_register3Index;

    uint8_t m_v1;
    uint8_t m_v2;
    uint16_t m_vX;

    uint8_t m_operationCache;
    uint8_t m_dataCache;
    uint16_t m_addressCache;

    HbcMotherboard *m_motherboard;
};

#endif // CPU_H
