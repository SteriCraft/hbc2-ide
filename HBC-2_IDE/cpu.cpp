#include "motherboard.h"

HbcCpu* HbcCpu::m_singleton = nullptr;


// PUBLIC
HbcCpu* HbcCpu::getInstance(HbcMotherboard *motherboard)
{
    if (m_singleton == nullptr)
        m_singleton = new HbcCpu(motherboard);

    return m_singleton;
}

void HbcCpu::tick()
{
    // TODO
    // Interruption management

    if (!m_flags[(int)CPU::Flags::HALT])
    {
        fetch();

        execute();

        if (!m_jumpOccured)
            m_programCounter += INSTRUCTION_SIZE;
        else
            m_jumpOccured = false;
    }
}


// PRIVATE
HbcCpu::HbcCpu(HbcMotherboard *motherboard)
{
    m_motherboard = motherboard;

    init();
}

void HbcCpu::init()
{
    for (unsigned int i(0); i < CPU_REGISTER_NB; i++)
        m_registers[i] = 0x00;

    for (unsigned int i(0); i < FLAGS_NB; i++)
        m_flags[i] = (i == (int)CPU::Flags::INTERRUPT) ? true : false; // Interrupt flag up by default

    m_instructionRegister = 0x00000000;

    m_jumpOccured = false;
    m_programCounter = 0x0000;
    m_stackPointer = 0x00;
    m_stackIsFull = false;

    m_opcode = CPU::InstrOpcode::NOP;
    m_addressingMode = CPU::AddrMode::NONE;

    m_register1Index = CPU::Register::A;
    m_register2Index = CPU::Register::A;
    m_register3Index = CPU::Register::A;

    m_v1 = 0x00;
    m_v2 = 0x00;
    m_vX = 0x0000;

    m_operationCache = 0x00;
    m_dataCache = 0x00;
    m_addressCache = 0x0000;
}

void HbcCpu::fetch()
{
    // Draw instruction from memory
    m_instructionRegister =  ((uint32_t)m_motherboard->readRam(m_programCounter))     << 24;
    m_instructionRegister += ((uint32_t)m_motherboard->readRam(m_programCounter + 1)) << 16;
    m_instructionRegister += ((uint32_t)m_motherboard->readRam(m_programCounter + 2)) << 8;
    m_instructionRegister +=            m_motherboard->readRam(m_programCounter + 3);

    // Analyse instruction
    m_opcode = (CPU::InstrOpcode)((m_instructionRegister & OPCODE_MASK) >> 26);
    m_addressingMode = (CPU::AddrMode)((m_instructionRegister & ADDRMODE_MASK) >> 22);

    m_register1Index = (CPU::Register)((m_instructionRegister & R1_MASK) >> 19);
    m_register2Index = (CPU::Register)((m_instructionRegister & R2_MASK) >> 16);
    m_register2Index = (CPU::Register)((m_instructionRegister & R3_MASK) >> 8);

    m_v1 = (m_instructionRegister & V1_MASK) >> 8;
    m_v2 = m_instructionRegister & V2_MASK;
    m_vX = m_instructionRegister & VX_MASK;
}

void HbcCpu::execute()
{
    // TODO
    // Modify flags when affected by instruction
    // Implement addressing modes

    switch (m_opcode)
    {
        case CPU::InstrOpcode::ADC:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] + m_registers[(int)m_register2Index] + 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] + m_registers[(int)m_register2Index] + 1) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_operationCache += m_registers[(int)m_register1Index] + m_v1 + 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] + m_v1 + 1) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;

            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_operationCache = m_registers[(int)m_register1Index] + m_dataCache + 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] + m_dataCache + 1) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            break;

        case CPU::InstrOpcode::ADD:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache += m_registers[(int)m_register1Index] + m_registers[(int)m_register2Index];

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] + m_registers[(int)m_register2Index]) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_operationCache += m_registers[(int)m_register1Index] + m_v1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] + m_v1) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_operationCache = m_registers[(int)m_register1Index] + m_dataCache;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] + m_dataCache) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            break;

        case CPU::InstrOpcode::AND:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] & m_registers[(int)m_register2Index];

                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_operationCache = m_registers[(int)m_register1Index] & m_v1;

                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_registers[(int)m_register1Index] &= m_dataCache;

                m_flags[(int)CPU::Flags::ZERO] = m_registers[(int)m_register1Index] == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_registers[(int)m_register1Index] & 0x80;
            }
            break;

        case CPU::InstrOpcode::CAL:
            if (m_addressingMode == CPU::AddrMode::REG16)
            {
                m_addressCache = ((uint16_t)m_registers[(int)m_register1Index] << 8) + m_registers[(int)m_register2Index];

                push((uint8_t)(m_programCounter >> 8));     // MSB
                push((uint8_t)(m_programCounter & 0x00FF)); // LSB

                m_programCounter = m_addressCache;

                m_jumpOccured = true;
            }
            else if (m_addressingMode == CPU::AddrMode::IMM16)
            {
                push((uint8_t)(m_programCounter >> 8));     // MSB
                push((uint8_t)(m_programCounter & 0x00FF)); // LSB

                m_programCounter = m_vX;

                m_jumpOccured = true;
            }
            break;

        case CPU::InstrOpcode::CLC:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::CARRY] = false;
            break;

        case CPU::InstrOpcode::CLE:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::EQUAL] = false;
            break;

        case CPU::InstrOpcode::CLI:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::INTERRUPT] = false;
            break;

        case CPU::InstrOpcode::CLN:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::NEGATIVE] = false;
            break;

        case CPU::InstrOpcode::CLS:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::SUPERIOR] = false;
            break;

        case CPU::InstrOpcode::CLZ:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::ZERO] = false;
            break;

        case CPU::InstrOpcode::CLF:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::INFERIOR] = false;
            break;

        case CPU::InstrOpcode::CMP:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_flags[(int)CPU::Flags::SUPERIOR] = m_registers[(int)m_register1Index] >  m_registers[(int)m_register2Index];
                m_flags[(int)CPU::Flags::EQUAL] =    m_registers[(int)m_register1Index] == m_registers[(int)m_register2Index];
                m_flags[(int)CPU::Flags::INFERIOR] = m_registers[(int)m_register1Index] <  m_registers[(int)m_register2Index];
                // TODO : Update documentation because flag ZERO don't get updated
                // TODO : Update document because it's not flag "I" (interrupt) but "F" (inferior) that gets updated
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_flags[(int)CPU::Flags::SUPERIOR] = m_registers[(int)m_register1Index] >  m_v1;
                m_flags[(int)CPU::Flags::EQUAL] =    m_registers[(int)m_register1Index] == m_v1;
                m_flags[(int)CPU::Flags::INFERIOR] = m_registers[(int)m_register1Index] <  m_v1;
            }
            else if (m_addressingMode == CPU::AddrMode::RAMREG_IMMREG)
            {
                m_addressCache = ((uint16_t)m_registers[(int)m_register1Index] << 8) + m_registers[(int)m_register2Index];
                m_dataCache = m_motherboard->readRam(m_addressCache);

                m_flags[(int)CPU::Flags::SUPERIOR] = m_registers[(int)m_register3Index] >  m_dataCache;
                m_flags[(int)CPU::Flags::EQUAL]    = m_registers[(int)m_register3Index] == m_dataCache;
                m_flags[(int)CPU::Flags::INFERIOR] = m_registers[(int)m_register3Index] <  m_dataCache;
            }
            break;

        case CPU::InstrOpcode::DEC:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] - 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] - 1) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG16)
            {
                m_addressCache = ((uint16_t)m_registers[(int)m_register1Index] << 8) + m_registers[(int)m_register2Index];
                m_dataCache = m_motherboard->readRam(m_addressCache);
                m_operationCache = m_dataCache - 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_dataCache - 1) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_motherboard->writeRam(m_addressCache, m_operationCache);
            }
            else if (m_addressingMode == CPU::AddrMode::IMM16)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_operationCache = m_dataCache - 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_dataCache - 1) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_motherboard->writeRam(m_vX, m_operationCache);
            }
            break;

        case CPU::InstrOpcode::HLT:
            m_flags[(int)CPU::Flags::CARRY] = true;
            break;

        case CPU::InstrOpcode::IN: // TODO
        case CPU::InstrOpcode::OUT: // TODO
        case CPU::InstrOpcode::INC:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] + 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] + 1) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG16)
            {
                m_addressCache = ((uint16_t)m_registers[(int)m_register1Index] << 8) + m_registers[(int)m_register2Index];
                m_dataCache = m_motherboard->readRam(m_addressCache);
                m_operationCache = m_dataCache + 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_dataCache + 1) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_motherboard->writeRam(m_addressCache, m_operationCache);
            }
            else if (m_addressingMode == CPU::AddrMode::IMM16)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_operationCache = m_dataCache + 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_dataCache + 1) > 0xFF;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_motherboard->writeRam(m_vX, m_operationCache);
            }
            break;

        case CPU::InstrOpcode::INT: // TODO
        case CPU::InstrOpcode::IRT: // TODO
        case CPU::InstrOpcode::JMC:
            if (m_flags[(int)CPU::Flags::CARRY])
            {
                jump();
            }
            break;

        case CPU::InstrOpcode::JME:
            if (m_flags[(int)CPU::Flags::EQUAL])
            {
                jump();
            }
            break;

        case CPU::InstrOpcode::JMN:
            if (m_flags[(int)CPU::Flags::NEGATIVE])
            {
                jump();
            }
            break;

        case CPU::InstrOpcode::JMP:
            jump();
            break;

        case CPU::InstrOpcode::JMS:
            if (m_flags[(int)CPU::Flags::SUPERIOR])
            {
                jump();
            }
            break;

        case CPU::InstrOpcode::JMZ:
            if (m_flags[(int)CPU::Flags::ZERO])
            {
                jump();
            }
            break;

        case CPU::InstrOpcode::JMF:
            if (m_flags[(int)CPU::Flags::INFERIOR])
            {
                jump();
            }
            break;

        case CPU::InstrOpcode::STR:
            if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_motherboard->writeRam(m_vX, m_registers[(int)m_register1Index]);
            }
            else if (m_addressingMode == CPU::AddrMode::RAMREG_IMMREG)
            {
                m_addressCache = ((uint16_t)m_registers[(int)m_register1Index] << 8) + m_registers[(int)m_register2Index];
                m_motherboard->writeRam(m_addressCache, m_registers[(int)m_register3Index]);
            }
            break;

        case CPU::InstrOpcode::LOD:
            if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_registers[(int)m_register1Index] = m_motherboard->readRam(m_vX);
            }
            else if (m_addressingMode == CPU::AddrMode::RAMREG_IMMREG)
            {
                m_addressCache = ((uint16_t)m_registers[(int)m_register1Index] << 8) + m_registers[(int)m_register2Index];
                m_registers[(int)m_register3Index] = m_motherboard->readRam(m_addressCache);
            }
            break;

        case CPU::InstrOpcode::MOV:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] = m_registers[(int)m_register2Index];
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] = m_v1;
            }
            break;

        case CPU::InstrOpcode::NOT:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] = ~m_registers[(int)m_register1Index];

                m_flags[(int)CPU::Flags::ZERO] = m_registers[(int)m_register1Index] == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_registers[(int)m_register1Index] & 0x80;
            }
            else if (m_addressingMode == CPU::AddrMode::IMM16)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_operationCache = ~m_dataCache;

                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_motherboard->writeRam(m_vX, m_operationCache);
            }
            break;

        case CPU::InstrOpcode::OR:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] |= m_registers[(int)m_register2Index];

                m_flags[(int)CPU::Flags::ZERO] = m_registers[(int)m_register1Index] == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_registers[(int)m_register1Index] & 0x80;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] |= m_v1;

                m_flags[(int)CPU::Flags::ZERO] = m_registers[(int)m_register1Index] == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_registers[(int)m_register1Index] & 0x80;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_registers[(int)m_register1Index] |= m_dataCache;

                m_flags[(int)CPU::Flags::ZERO] = m_registers[(int)m_register1Index] == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_registers[(int)m_register1Index] & 0x80;
            }
            break;

        case CPU::InstrOpcode::POP:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                pop(m_registers[(int)m_register1Index]);
            }
            break;

        case CPU::InstrOpcode::PSH:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                push(m_registers[(int)m_register1Index]);
            }
            break;
        case CPU::InstrOpcode::RET:
            if (m_addressingMode == CPU::AddrMode::NONE)
            {
                pop(m_dataCache); // LSB
                m_addressCache = m_dataCache;

                pop(m_dataCache); // MSB
                m_addressCache += m_dataCache << 8;

                m_programCounter = m_addressCache;

                m_jumpOccured = true;
            }
            break;

        case CPU::InstrOpcode::SHL:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] << 1;

                m_flags[(int)CPU::Flags::CARRY] = m_registers[(int)m_register1Index] & 0x80;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            break;

        case CPU::InstrOpcode::ASR:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] >> 1;
                m_operationCache |= m_registers[(int)m_register1Index] & 0x80; // Keeps sign flag

                m_flags[(int)CPU::Flags::CARRY] = m_registers[(int)m_register1Index] & 0x01;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            break;

        case CPU::InstrOpcode::SHR:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] >> 1;

                m_flags[(int)CPU::Flags::CARRY] = m_registers[(int)m_register1Index] & 0x01;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                // TODO update documentation because negative flag would never be set by a right shift

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            break;

        case CPU::InstrOpcode::STC:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::CARRY] = true;
            break;

        case CPU::InstrOpcode::STE:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::EQUAL] = true;
            break;

        case CPU::InstrOpcode::STI:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::INTERRUPT] = true;
            break;

        case CPU::InstrOpcode::STN:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::NEGATIVE] = true;
            break;

        case CPU::InstrOpcode::STS:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::SUPERIOR] = true;
            break;

        case CPU::InstrOpcode::STZ:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::ZERO] = true;
            break;

        case CPU::InstrOpcode::STF:
            if (m_addressingMode == CPU::AddrMode::NONE)
                m_flags[(int)CPU::Flags::INFERIOR] = true;
            break;

        case CPU::InstrOpcode::SUB:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] - m_registers[(int)m_register2Index];

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] - m_registers[(int)m_register2Index]) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_operationCache = m_registers[(int)m_register1Index] - m_v1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] - m_v1) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_operationCache = m_registers[(int)m_register1Index] - m_dataCache;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] - m_v1) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            break;

        case CPU::InstrOpcode::SBB:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] - m_registers[(int)m_register2Index] - 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] - m_registers[(int)m_register2Index] - 1) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_operationCache = m_registers[(int)m_register1Index] - m_v1 - 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] - m_v1 - 1) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_operationCache = m_registers[(int)m_register1Index] - m_dataCache - 1;

                m_flags[(int)CPU::Flags::CARRY] = ((int)m_registers[(int)m_register1Index] - m_v1 - 1) < 0;
                m_flags[(int)CPU::Flags::ZERO] = m_operationCache == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_operationCache & 0x80;

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            break;

        case CPU::InstrOpcode::XOR:
            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] ^= m_registers[(int)m_register2Index];

                m_flags[(int)CPU::Flags::ZERO] = m_registers[(int)m_register1Index] == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_registers[(int)m_register1Index] & 0x80;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] ^= m_v1;

                m_flags[(int)CPU::Flags::ZERO] = m_registers[(int)m_register1Index] == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_registers[(int)m_register1Index] & 0x80;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                m_dataCache = m_motherboard->readRam(m_vX);
                m_registers[(int)m_register1Index] ^= m_dataCache;

                m_flags[(int)CPU::Flags::ZERO] = m_registers[(int)m_register1Index] == 0x00;
                m_flags[(int)CPU::Flags::NEGATIVE] = m_registers[(int)m_register1Index] & 0x80;
            }
            break;

        default:
            break;
    }
}

void HbcCpu::jump()
{
    if (m_addressingMode == CPU::AddrMode::REG16)
    {
            m_programCounter = ((uint16_t)m_registers[(int)m_register1Index] << 8) + m_registers[(int)m_register2Index];
            m_jumpOccured = true;
    }
    else if (m_addressingMode == CPU::AddrMode::IMM16)
    {
            m_programCounter = m_vX;
            m_jumpOccured = true;
    }
}

bool HbcCpu::pop(uint8_t &data)
{
    if (m_stackPointer == 0x00)
    {
        m_flags[(int)CPU::Flags::CARRY] = true;
    }
    else
    {
        m_stackPointer--;
        m_stackIsFull = false;

        data = m_motherboard->readRam(m_stackPointer);
    }
}

bool HbcCpu::push(uint8_t data)
{
    if (m_stackIsFull)
    {
        m_flags[(int)CPU::Flags::CARRY] = true;

        return true;
    }
    else
    {
        m_motherboard->writeRam(m_stackPointer, data);

        m_stackPointer++;
        m_stackIsFull = m_stackPointer == 0x00;

        return false;
    }
}
