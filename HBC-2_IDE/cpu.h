#ifndef CPU_H
#define CPU_H

/*!
 * \file cpu.h
 * \brief HBC-2 Central Processing Unit implementation
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QString>
#include <QDebug>
#include "computerDetails.h"

struct HbcMotherboard;

/*!
 * \struct HbcCpu
 * \brief Stores the Cpu state
 */
struct HbcCpu
{
    Byte m_registers[Cpu::REGISTERS_NB]; //!< Registers data
    bool m_flags[Cpu::FLAGS_NB]; //!< Control flags
    Dword m_instructionRegister; //!< 32-bit instruction register

    bool m_jumpOccured; //!< Used internaly
    Word m_programCounter; //!< 16-bit pointer to the current instruction
    Byte m_stackPointer; //!< 8-bit pointer to the current position in the stack
    bool m_stackIsFull; //!< Used internaly <i>(no flag for stack overflow or underflow)</i>

    Cpu::InstructionOpcode m_opcode; //!< Current instruction opcode <i>(after decoding)</i>
    Cpu::AddressingMode m_addressingMode; //!< Current instruction addressing mode <i>(after decoding)</i>

    Cpu::Register m_register1Index; //!< R1 pointer to a register <i>(after decoding)</i>
    Cpu::Register m_register2Index; //!< R2 pointer to a register <i>(after decoding)</i>
    Cpu::Register m_register3Index; //!< R3 pointer to a register <i>(after decoding)</i>

    Byte m_v1; //!< 8-bit value <i>(after decoding)</i>
    Byte m_v2; //!< 8-bit value <i>(after decoding)</i>
    Word m_vX; //!< 16-bit value (V1<<V2) <i>(after decoding)</i>

    Byte m_operationCache; //!< Used internaly
    Byte m_dataCache; //!< Used internaly
    Word m_addressCache; //!< Used internaly

    Cpu::CpuState m_currentState; //!< Used internaly to decode interrupts
    HbcMotherboard *m_motherboard; //!< Used internaly
};

// Already documented in computerDetails.h
namespace Cpu
{
    void init(HbcCpu &cpu, HbcMotherboard* mb); //*! Initializes the CPU

    /*!
     * \brief <b>Used internaly: </b> Step 1 of Cpu::tick
     *
     * \param cpu Reference to HbcCpu
     */
    void tick(HbcCpu &cpu); //!< Executes one instruction completely

    /*!
     * \brief <b>Used internaly: </b> Step 1 of Cpu::tick
     *
     * \param cpu Reference to HbcCpu
     */
    void fetch(HbcCpu &cpu);

    /*!
     * \brief <b>Used internaly: </b> Step 2 of Cpu::tick
     *
     * \param cpu Reference to HbcCpu
     */
    void decode(HbcCpu &cpu);

    /*!
     * \brief <b>Used internaly: </b> Step 3 of Cpu::tick
     *
     * \param cpu Reference to HbcCpu
     */
    void execute(HbcCpu &cpu);

    /*!
     * \brief <b>Used internaly: </b> Performs a jump <i>(out of Cpu::execute because this code is used often)</o>
     *
     * \param cpu Reference to HbcCpu
     */
    void jump(HbcCpu &cpu);

    /*!
     * \brief <b>Used internaly: </b> Pops last value on the stack <i>(no flag in case of underflow)</i>
     *
     * \param cpu Reference to HbcCpu
     * \param data Reference to store to popped value
     */
    void pop(HbcCpu &cpu, Byte &data);

    /*!
     * \brief <b>Used internaly: </b> Pushes a value on the stack <i>(no flag in case of overflow)</i>
     *
     * \param cpu Reference to HbcCpu
     * \param data Value to store in the stack
     */
    void push(HbcCpu &cpu, Byte data);
}

#endif // CPU_H
