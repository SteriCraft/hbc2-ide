#ifndef CPU_H
#define CPU_H

#include <QString>
#include <QDebug>

#include "computerDetails.h"

struct HbcMotherboard;

struct HbcCpu
{
    Byte m_registers[REGISTERS_NB];
    bool m_flags[FLAGS_NB];
    Dword m_instructionRegister;

    bool m_jumpOccured;
    Word m_programCounter;
    Byte m_stackPointer;
    bool m_stackIsFull;

    Computer::InstructionOpcode m_opcode;
    Computer::AddressingMode m_addressingMode;

    Computer::Register m_register1Index;
    Computer::Register m_register2Index;
    Computer::Register m_register3Index;

    Byte m_v1;
    Byte m_v2;
    Word m_vX;

    Byte m_operationCache;
    Byte m_dataCache;
    Word m_addressCache;

    Computer::CpuState m_currentState;
    HbcMotherboard *m_motherboard;
};

namespace Cpu
{
    void init(HbcCpu &cpu, HbcMotherboard* mb);
    void tick(HbcCpu &cpu); // Executes one instruction completely

    void fetch(HbcCpu &cpu);
    void decode(HbcCpu &cpu);
    void execute(HbcCpu &cpu);

    void jump(HbcCpu &cpu);
    bool pop(HbcCpu &cpu, Byte &data); // Returns true if stack underflows
    bool push(HbcCpu &cpu, Byte data); // Returns true if stack overflows
}

#endif // CPU_H
