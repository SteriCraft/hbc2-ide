#ifndef CPU_H
#define CPU_H

#include <QString>
#include <QDebug>

#include "cpuDetails.h"

struct HbcMotherboard;

struct HbcCpu
{
    uint8_t m_registers[REGISTERS_NB];
    bool m_flags[FLAGS_NB];
    uint32_t m_instructionRegister;

    bool m_jumpOccured;
    uint16_t m_programCounter;
    uint8_t m_stackPointer;
    bool m_stackIsFull;

    CPU::InstructionOpcode m_opcode;
    CPU::AddressingMode m_addressingMode;

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

namespace Cpu
{
    void init(HbcCpu &cpu, HbcMotherboard* mb);
    void tick(HbcCpu &cpu); // Executes one instruction completely

    void fetch(HbcCpu &cpu);
    void execute(HbcCpu &cpu);

    void jump(HbcCpu &cpu);
    bool pop(HbcCpu &cpu, uint8_t &data); // Returns true if stack underflows
    bool push(HbcCpu &cpu, uint8_t data); // Returns true if stack overflows
}

#endif // CPU_H
