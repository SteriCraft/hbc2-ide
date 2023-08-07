#include "motherboard.h"

void Cpu::init(HbcCpu &cpu, HbcMotherboard* mb)
{
    for (unsigned int i(0); i < REGISTERS_NB; i++)
        cpu.m_registers[i] = 0x00;

    for (unsigned int i(0); i < FLAGS_NB; i++)
        cpu.m_flags[i] = (i == (int)CPU::Flags::INTERRUPT) ? true : false; // Interrupt flag up by default

    cpu.m_instructionRegister = 0x00000000;

    cpu.m_jumpOccured = false;
    cpu.m_programCounter = PROGRAM_START_ADDRESS;
    cpu.m_stackPointer = 0x00;
    cpu.m_stackIsFull = false;

    cpu.m_opcode = CPU::InstructionOpcode::NOP;
    cpu.m_addressingMode = CPU::AddressingMode::NONE;

    cpu.m_register1Index = CPU::Register::A;
    cpu.m_register2Index = CPU::Register::A;
    cpu.m_register3Index = CPU::Register::A;

    cpu.m_v1 = 0x00;
    cpu.m_v2 = 0x00;
    cpu.m_vX = 0x0000;

    cpu.m_operationCache = 0x00;
    cpu.m_dataCache = 0x00;
    cpu.m_addressCache = 0x0000;

    cpu.m_motherboard = mb;
}

void Cpu::tick(HbcCpu &cpu)
{
    // TODO
    // Interruption management
    if (!cpu.m_flags[(int)CPU::Flags::HALT])
    {
        Cpu::fetch(cpu);

        Cpu::execute(cpu);

        if (!cpu.m_jumpOccured)
            cpu.m_programCounter += INSTRUCTION_SIZE;
        else
            cpu.m_jumpOccured = false;
    }
}

void Cpu::fetch(HbcCpu &cpu)
{
    // Draw instruction from memory
    cpu.m_instructionRegister =  ((uint32_t)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter))     << 24;
    cpu.m_instructionRegister += ((uint32_t)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 1)) << 16;
    cpu.m_instructionRegister += ((uint32_t)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 2)) << 8;
    cpu.m_instructionRegister +=            Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 3);

    // Analyse instruction
    cpu.m_opcode = (CPU::InstructionOpcode)((cpu.m_instructionRegister & OPCODE_MASK) >> 26);
    cpu.m_addressingMode = (CPU::AddressingMode)((cpu.m_instructionRegister & ADDRMODE_MASK) >> 22);

    cpu.m_register1Index = (CPU::Register)((cpu.m_instructionRegister & R1_MASK) >> 19);
    cpu.m_register2Index = (CPU::Register)((cpu.m_instructionRegister & R2_MASK) >> 16);
    cpu.m_register2Index = (CPU::Register)((cpu.m_instructionRegister & R3_MASK) >> 8);

    cpu.m_v1 = (cpu.m_instructionRegister & V1_MASK) >> 8;
    cpu.m_v2 = cpu.m_instructionRegister & V2_MASK;
    cpu.m_vX = cpu.m_instructionRegister & VX_MASK;
}

void Cpu::execute(HbcCpu &cpu)
{
    // TODO
    // Modify flags when affected by instruction
    // Implement addressing modes

    switch (cpu.m_opcode)
    {
        case CPU::InstructionOpcode::ADC:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index] + 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index] + 1) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache += cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1 + 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1 + 1) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;

            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache + 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case CPU::InstructionOpcode::ADD:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache += cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index]) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache += cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case CPU::InstructionOpcode::AND:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] & cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] & cpu.m_v1;

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] &= cpu.m_dataCache;

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        case CPU::InstructionOpcode::CAL:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((uint16_t)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];

                Cpu::push(cpu, (uint8_t)(cpu.m_programCounter >> 8));     // MSB
                Cpu::push(cpu, (uint8_t)(cpu.m_programCounter & 0x00FF)); // LSB

                cpu.m_programCounter = cpu.m_addressCache;

                cpu.m_jumpOccured = true;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::IMM16)
            {
                Cpu::push(cpu, (uint8_t)(cpu.m_programCounter >> 8));     // MSB
                Cpu::push(cpu, (uint8_t)(cpu.m_programCounter & 0x00FF)); // LSB

                cpu.m_programCounter = cpu.m_vX;

                cpu.m_jumpOccured = true;
            }
            break;

        case CPU::InstructionOpcode::CLC:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::CARRY] = false;
            break;

        case CPU::InstructionOpcode::CLE:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::EQUAL] = false;
            break;

        case CPU::InstructionOpcode::CLI:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::INTERRUPT] = false;
            break;

        case CPU::InstructionOpcode::CLN:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = false;
            break;

        case CPU::InstructionOpcode::CLS:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::SUPERIOR] = false;
            break;

        case CPU::InstructionOpcode::CLZ:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::ZERO] = false;
            break;

        case CPU::InstructionOpcode::CLF:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::INFERIOR] = false;
            break;

        case CPU::InstructionOpcode::CMP:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_flags[(int)CPU::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register1Index] >  cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_flags[(int)CPU::Flags::EQUAL] =    cpu.m_registers[(int)cpu.m_register1Index] == cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_flags[(int)CPU::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register1Index] <  cpu.m_registers[(int)cpu.m_register2Index];
                // TODO : Update documentation because flag ZERO don't get updated
                // TODO : Update document because it's not flag "I" (interrupt) but "F" (inferior) that gets updated
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_flags[(int)CPU::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register1Index] >  cpu.m_v1;
                cpu.m_flags[(int)CPU::Flags::EQUAL] =    cpu.m_registers[(int)cpu.m_register1Index] == cpu.m_v1;
                cpu.m_flags[(int)CPU::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register1Index] <  cpu.m_v1;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((uint16_t)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);

                cpu.m_flags[(int)CPU::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register3Index] >  cpu.m_dataCache;
                cpu.m_flags[(int)CPU::Flags::EQUAL]    = cpu.m_registers[(int)cpu.m_register3Index] == cpu.m_dataCache;
                cpu.m_flags[(int)CPU::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register3Index] <  cpu.m_dataCache;
            }
            break;

        case CPU::InstructionOpcode::DEC:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - 1) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((uint16_t)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
                cpu.m_operationCache = cpu.m_dataCache - 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_dataCache - 1) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_operationCache);
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_dataCache - 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_dataCache - 1) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case CPU::InstructionOpcode::HLT:
            cpu.m_flags[(int)CPU::Flags::CARRY] = true;
            break;

        case CPU::InstructionOpcode::IN: // TODO
        case CPU::InstructionOpcode::OUT: // TODO
        case CPU::InstructionOpcode::INC:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + 1) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((uint16_t)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
                cpu.m_operationCache = cpu.m_dataCache + 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_operationCache);
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_dataCache + 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case CPU::InstructionOpcode::INT: // TODO
        case CPU::InstructionOpcode::IRT: // TODO
        case CPU::InstructionOpcode::JMC:
            if (cpu.m_flags[(int)CPU::Flags::CARRY])
            {
                Cpu::jump(cpu);
            }
            break;

        case CPU::InstructionOpcode::JME:
            if (cpu.m_flags[(int)CPU::Flags::EQUAL])
            {
                Cpu::jump(cpu);
            }
            break;

        case CPU::InstructionOpcode::JMN:
            if (cpu.m_flags[(int)CPU::Flags::NEGATIVE])
            {
                Cpu::jump(cpu);
            }
            break;

        case CPU::InstructionOpcode::JMP:
            Cpu::jump(cpu);
            break;

        case CPU::InstructionOpcode::JMS:
            if (cpu.m_flags[(int)CPU::Flags::SUPERIOR])
            {
                Cpu::jump(cpu);
            }
            break;

        case CPU::InstructionOpcode::JMZ:
            if (cpu.m_flags[(int)CPU::Flags::ZERO])
            {
                Cpu::jump(cpu);
            }
            break;

        case CPU::InstructionOpcode::JMF:
            if (cpu.m_flags[(int)CPU::Flags::INFERIOR])
            {
                Cpu::jump(cpu);
            }
            break;

        case CPU::InstructionOpcode::STR:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((uint16_t)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_registers[(int)cpu.m_register3Index]);
            }
            break;

        case CPU::InstructionOpcode::LOD:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((uint16_t)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_registers[(int)cpu.m_register3Index] = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
            }
            break;

        case CPU::InstructionOpcode::MOV:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_registers[(int)cpu.m_register2Index];
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_v1;
            }
            break;

        case CPU::InstructionOpcode::NOT:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = ~cpu.m_registers[(int)cpu.m_register1Index];

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = ~cpu.m_dataCache;

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case CPU::InstructionOpcode::OR:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_v1;

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_dataCache;

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        case CPU::InstructionOpcode::POP:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                Cpu::pop(cpu, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            break;

        case CPU::InstructionOpcode::PSH:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                Cpu::push(cpu, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            break;
        case CPU::InstructionOpcode::RET:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
            {
                Cpu::pop(cpu, cpu.m_dataCache); // LSB
                cpu.m_addressCache = cpu.m_dataCache;

                Cpu::pop(cpu, cpu.m_dataCache); // MSB
                cpu.m_addressCache += cpu.m_dataCache << 8;

                cpu.m_programCounter = cpu.m_addressCache;

                cpu.m_jumpOccured = true;
            }
            break;

        case CPU::InstructionOpcode::SHL:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] << 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case CPU::InstructionOpcode::ASR:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] >> 1;
                cpu.m_operationCache |= cpu.m_registers[(int)cpu.m_register1Index] & 0x80; // Keeps sign flag

                cpu.m_flags[(int)CPU::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x01;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case CPU::InstructionOpcode::SHR:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] >> 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x01;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                // TODO update documentation because negative flag would never be set by a right shift

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case CPU::InstructionOpcode::STC:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::CARRY] = true;
            break;

        case CPU::InstructionOpcode::STE:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::EQUAL] = true;
            break;

        case CPU::InstructionOpcode::STI:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::INTERRUPT] = true;
            break;

        case CPU::InstructionOpcode::STN:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = true;
            break;

        case CPU::InstructionOpcode::STS:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::SUPERIOR] = true;
            break;

        case CPU::InstructionOpcode::STZ:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::ZERO] = true;
            break;

        case CPU::InstructionOpcode::STF:
            if (cpu.m_addressingMode == CPU::AddressingMode::NONE)
                cpu.m_flags[(int)CPU::Flags::INFERIOR] = true;
            break;

        case CPU::InstructionOpcode::SUB:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index]) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_dataCache;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case CPU::InstructionOpcode::SBB:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index] - 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index] - 1) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1 - 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1 - 1) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_dataCache - 1;

                cpu.m_flags[(int)CPU::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1 - 1) < 0;
                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case CPU::InstructionOpcode::XOR:
            if (cpu.m_addressingMode == CPU::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_v1;

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == CPU::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_dataCache;

                cpu.m_flags[(int)CPU::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)CPU::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        default:
            break;
    }
}

void Cpu::jump(HbcCpu &cpu)
{
    if (cpu.m_addressingMode == CPU::AddressingMode::REG16)
    {
            cpu.m_programCounter = ((uint16_t)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
            cpu.m_jumpOccured = true;
    }
    else if (cpu.m_addressingMode == CPU::AddressingMode::IMM16)
    {
            cpu.m_programCounter = cpu.m_vX;
            cpu.m_jumpOccured = true;
    }
}

bool Cpu::pop(HbcCpu &cpu, uint8_t &data)
{
    if (cpu.m_stackPointer == 0x00)
    {
        cpu.m_flags[(int)CPU::Flags::CARRY] = true;

        return true;
    }
    else
    {
        cpu.m_stackPointer--;
        cpu.m_stackIsFull = false;

        data = Motherboard::readRam(*cpu.m_motherboard, cpu.m_stackPointer);

        return false;
    }
}

bool Cpu::push(HbcCpu &cpu, uint8_t data)
{
    if (cpu.m_stackIsFull)
    {
        cpu.m_flags[(int)CPU::Flags::CARRY] = true;

        return true;
    }
    else
    {
        Motherboard::writeRam(*cpu.m_motherboard, cpu.m_stackPointer, data);

        cpu.m_stackPointer++;
        cpu.m_stackIsFull = cpu.m_stackPointer == 0x00;

        return false;
    }
}
