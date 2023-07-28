#include "cpu.h"

hbcCpu* hbcCpu::m_singleton = nullptr;


// PUBLIC
hbcCpu* hbcCpu::getInstance()
{
    if (m_singleton == nullptr)
        m_singleton = new hbcCpu();

    return m_singleton;
}

void hbcCpu::tick()
{
    // TODO
    // Interruption management

    fetch();

    execute();

    if (!m_jumpOccured)
        m_programCounter += INSTRUCTION_SIZE;
    else
        m_jumpOccured = false;
}


// PRIVATE
hbcCpu::hbcCpu()
{
    init();
}

void hbcCpu::init()
{
    for (unsigned int i(0); i < CPU_REGISTER_NB; i++)
        m_registers[i] = 0x00;

    for (unsigned int i(0); i < FLAGS_NB; i++)
        m_flags[i] = (i == (int)CPU::Flags::INTERRUPT) ? true : false; // Interrupt flag up by default

    m_instructionRegister = 0x00000000;

    m_jumpOccured = false;
    m_programCounter = 0x0000;
    m_stackPointer = 0x00;

    m_opcode = CPU::InstrOpcode::NOP;
    m_addressingMode = CPU::AddrMode::NONE;

    m_register1Index = CPU::Register::A;
    m_register2Index = CPU::Register::A;

    m_v1 = 0x00;
    m_v2 = 0x00;
    m_vX = 0x0000;

    m_operationCache = 0x00;
}

void hbcCpu::fetch()
{
    // TODO
    // Draw instruction from memory

    m_opcode = (CPU::InstrOpcode)((m_instructionRegister & OPCODE_MASK) >> 26);
    m_addressingMode = (CPU::AddrMode)((m_instructionRegister & ADDRMODE_MASK) >> 22);

    m_register1Index = (CPU::Register)((m_instructionRegister & R1_MASK) >> 19);
    m_register2Index = (CPU::Register)((m_instructionRegister & R2_MASK) >> 16);

    m_v1 = (m_instructionRegister & V1_MASK) >> 8;
    m_v2 = m_instructionRegister & V2_MASK;
    m_vX = m_instructionRegister & VX_MASK;
}

void hbcCpu::execute()
{
    // TODO
    // Modify flags when affected by instruction
    // Implement addressing modes

    switch (m_opcode)
    {
        case CPU::InstrOpcode::ADC:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] += m_registers[(int)m_register2Index] + 1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] += m_v1 + 1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                // Retrieve data from memory
            }
            break;

        case CPU::InstrOpcode::ADD:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] += m_registers[(int)m_register2Index];
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] += m_v1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                // Retrieve data from memory
            }
            break;

        case CPU::InstrOpcode::AND:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] &= m_registers[(int)m_register2Index];
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] &= m_v1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                // Retrieve data from memory
            }
            break;

        case CPU::InstrOpcode::CAL:
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
        case CPU::InstrOpcode::DEC:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index]--;
            }
            else if (m_addressingMode == CPU::AddrMode::REG16)
            {
                // Retrieve data from ram
            }
            else if (m_addressingMode == CPU::AddrMode::IMM16)
            {
                // Retrieve data from ram
            }
            break;

        case CPU::InstrOpcode::HLT:
            m_flags[(int)CPU::Flags::CARRY] = true;
            break;

        case CPU::InstrOpcode::IN:
        case CPU::InstrOpcode::OUT:
        case CPU::InstrOpcode::INC:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index]++;
            }
            else if (m_addressingMode == CPU::AddrMode::REG16)
            {
                // Retrieve data from ram
            }
            else if (m_addressingMode == CPU::AddrMode::IMM16)
            {
                // Retrieve data from ram
            }
            break;

        case CPU::InstrOpcode::INT:
        case CPU::InstrOpcode::IRT:
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
        case CPU::InstrOpcode::LOD:
        case CPU::InstrOpcode::MOV:
        case CPU::InstrOpcode::NOT:
            m_registers[(int)m_register1Index] = ~m_registers[(int)m_register1Index];
            break;

        case CPU::InstrOpcode::OR:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] |= m_registers[(int)m_register2Index];
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] |= m_v1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                // Retrieve data from memory
            }
        case CPU::InstrOpcode::POP:
        case CPU::InstrOpcode::PSH:
        case CPU::InstrOpcode::RET:
        case CPU::InstrOpcode::SHL:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] = m_registers[(int)m_register1Index] << 1;
            }
            break;

        case CPU::InstrOpcode::ASR:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_operationCache = m_registers[(int)m_register1Index] >> 1;

                if ((m_registers[(int)m_register1Index] & 0x80) == 0x80) // If sign flag is set...
                {
                    m_operationCache |= (0x01 << 7); // ...sets the sign flag
                }

                // Implement flag consequence

                m_registers[(int)m_register1Index] = m_operationCache;
            }
            break;

        case CPU::InstrOpcode::SHR:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] = m_registers[(int)m_register1Index] >> 1;
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
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] -= m_registers[(int)m_register2Index];
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] -= m_v1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                // Retrieve data from memory
            }
            break;

        case CPU::InstrOpcode::SBB:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] -= m_registers[(int)m_register2Index] - 1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] -= m_v1 - 1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                // Retrieve data from memory
            }
            break;

        case CPU::InstrOpcode::XOR:
            // Implement flag consequence

            if (m_addressingMode == CPU::AddrMode::REG)
            {
                m_registers[(int)m_register1Index] ^= m_registers[(int)m_register2Index];
            }
            else if (m_addressingMode == CPU::AddrMode::REG_IMM8)
            {
                m_registers[(int)m_register1Index] ^= m_v1;
            }
            else if (m_addressingMode == CPU::AddrMode::REG_RAM)
            {
                // Retrieve data from memory
            }
            break;

        default:
            break;
    }
}

void hbcCpu::jump()
{
    if (m_addressingMode == CPU::AddrMode::REG16)
    {
            m_programCounter = ((uint16_t)m_registers[m_register1Index] << 8) + m_registers[m_register2Index];
            m_jumpOccured = true;
    }
    else if (m_addressingMode == CPU::AddrMode::IMM16)
    {
            m_programCounter = m_vX;
            m_jumpOccured = true;
    }
}
