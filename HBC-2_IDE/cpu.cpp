#include "motherboard.h"

void Cpu::init(HbcCpu &cpu, HbcMotherboard* mb)
{
    for (unsigned int i(0); i < REGISTERS_NB; i++)
        cpu.m_registers[i] = 0x00;

    for (unsigned int i(0); i < FLAGS_NB; i++)
        cpu.m_flags[i] = (i == (int)Computer::Flags::INTERRUPT) ? true : false; // Interrupt flag up by default

    cpu.m_instructionRegister = 0x00000000;

    cpu.m_jumpOccured = false;
    cpu.m_programCounter = PROGRAM_START_ADDRESS;
    cpu.m_stackPointer = 0x00;
    cpu.m_stackIsFull = false;

    cpu.m_opcode = Computer::InstructionOpcode::NOP;
    cpu.m_addressingMode = Computer::AddressingMode::NONE;

    cpu.m_register1Index = Computer::Register::A;
    cpu.m_register2Index = Computer::Register::A;
    cpu.m_register3Index = Computer::Register::A;

    cpu.m_v1 = 0x00;
    cpu.m_v2 = 0x00;
    cpu.m_vX = 0x0000;

    cpu.m_operationCache = 0x00;
    cpu.m_dataCache = 0x00;
    cpu.m_addressCache = 0x0000;

    cpu.m_currentState = Computer::CpuState::INSTRUCTION_EXEC;
    cpu.m_motherboard = mb;
}

void Cpu::tick(HbcCpu &cpu)
{
    switch (cpu.m_currentState)
    {
        case Computer::CpuState::INSTRUCTION_EXEC:
            if (cpu.m_flags[(int)Computer::Flags::INTERRUPT])
            {
                if (cpu.m_motherboard->m_int)
                {
                    cpu.m_motherboard->m_inr = true;

                    cpu.m_currentState = Computer::CpuState::INTERRUPT_MANAGEMENT;
                    break;
                }
            }

            if (!cpu.m_flags[(int)Computer::Flags::HALT])
            {
                Cpu::fetch(cpu);
                Cpu::decode(cpu);
                Cpu::execute(cpu);

                if (!cpu.m_jumpOccured)
                    cpu.m_programCounter += INSTRUCTION_SIZE;
                else
                    cpu.m_jumpOccured = false;
            }
            break;

        case Computer::CpuState::INTERRUPT_MANAGEMENT:
            if (!cpu.m_motherboard->m_int)
            {
                Cpu::push(cpu, cpu.m_registers[(int)Computer::Register::I]);

                Cpu::push(cpu, (Byte)(cpu.m_programCounter >> 8));   // LSB
                Cpu::push(cpu, (Byte)cpu.m_programCounter & 0x00FF); // MSB

                cpu.m_flags[(int)Computer::Flags::INTERRUPT] = false;
                cpu.m_flags[(int)Computer::Flags::HALT] = false;

                Word ivtAddress(IVT_START_ADDRESS + (cpu.m_motherboard->m_addressBus & INTERRUPT_PORT_MASK) * 2);

                cpu.m_programCounter = (Word)Motherboard::readRam(*cpu.m_motherboard, ivtAddress) << 8;
                cpu.m_programCounter += Motherboard::readRam(*cpu.m_motherboard, ivtAddress + 1);
                cpu.m_jumpOccured = true;

                cpu.m_currentState = Computer::CpuState::INSTRUCTION_EXEC;
            }
            break;
    }
}

void Cpu::fetch(HbcCpu &cpu)
{
    cpu.m_instructionRegister =  ((Dword)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter))     << 24;
    cpu.m_instructionRegister += ((Dword)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 1)) << 16;
    cpu.m_instructionRegister += ((Dword)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 2)) << 8;
    cpu.m_instructionRegister +=            Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 3);
}

void Cpu::decode(HbcCpu &cpu)
{
    cpu.m_opcode = (Computer::InstructionOpcode)((cpu.m_instructionRegister & OPCODE_MASK) >> 26);
    cpu.m_addressingMode = (Computer::AddressingMode)((cpu.m_instructionRegister & ADDRMODE_MASK) >> 22);

    cpu.m_register1Index = (Computer::Register)((cpu.m_instructionRegister & R1_MASK) >> 19);
    cpu.m_register2Index = (Computer::Register)((cpu.m_instructionRegister & R2_MASK) >> 16);
    cpu.m_register2Index = (Computer::Register)((cpu.m_instructionRegister & R3_MASK) >> 8);

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
        case Computer::InstructionOpcode::ADC:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index] + 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index] + 1) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache += cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1 + 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1 + 1) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;

            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache + 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Computer::InstructionOpcode::ADD:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache += cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index]) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache += cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Computer::InstructionOpcode::AND:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] & cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] & cpu.m_v1;

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] &= cpu.m_dataCache;

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        case Computer::InstructionOpcode::CAL:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];

                Cpu::push(cpu, (Byte)(cpu.m_programCounter >> 8));     // MSB
                Cpu::push(cpu, (Byte)(cpu.m_programCounter & 0x00FF)); // LSB

                cpu.m_programCounter = cpu.m_addressCache;

                cpu.m_jumpOccured = true;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::IMM16)
            {
                Cpu::push(cpu, (Byte)(cpu.m_programCounter >> 8));     // MSB
                Cpu::push(cpu, (Byte)(cpu.m_programCounter & 0x00FF)); // LSB

                cpu.m_programCounter = cpu.m_vX;

                cpu.m_jumpOccured = true;
            }
            break;

        case Computer::InstructionOpcode::CLC:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::CARRY] = false;
            break;

        case Computer::InstructionOpcode::CLE:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::EQUAL] = false;
            break;

        case Computer::InstructionOpcode::CLI:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::INTERRUPT] = false;
            break;

        case Computer::InstructionOpcode::CLN:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = false;
            break;

        case Computer::InstructionOpcode::CLS:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::SUPERIOR] = false;
            break;

        case Computer::InstructionOpcode::CLZ:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::ZERO] = false;
            break;

        case Computer::InstructionOpcode::CLF:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::INFERIOR] = false;
            break;

        case Computer::InstructionOpcode::CMP:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_flags[(int)Computer::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register1Index] >  cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_flags[(int)Computer::Flags::EQUAL] =    cpu.m_registers[(int)cpu.m_register1Index] == cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_flags[(int)Computer::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register1Index] <  cpu.m_registers[(int)cpu.m_register2Index];
                // TODO : Update documentation because flag ZERO don't get updated
                // TODO : Update document because it's not flag "I" (interrupt) but "F" (inferior) that gets updated
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_flags[(int)Computer::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register1Index] >  cpu.m_v1;
                cpu.m_flags[(int)Computer::Flags::EQUAL] =    cpu.m_registers[(int)cpu.m_register1Index] == cpu.m_v1;
                cpu.m_flags[(int)Computer::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register1Index] <  cpu.m_v1;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);

                cpu.m_flags[(int)Computer::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register3Index] >  cpu.m_dataCache;
                cpu.m_flags[(int)Computer::Flags::EQUAL]    = cpu.m_registers[(int)cpu.m_register3Index] == cpu.m_dataCache;
                cpu.m_flags[(int)Computer::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register3Index] <  cpu.m_dataCache;
            }
            break;

        case Computer::InstructionOpcode::DEC:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - 1) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
                cpu.m_operationCache = cpu.m_dataCache - 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_dataCache - 1) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_operationCache);
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_dataCache - 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_dataCache - 1) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case Computer::InstructionOpcode::HLT:
            cpu.m_flags[(int)Computer::Flags::HALT] = true;
            cpu.m_flags[(int)Computer::Flags::INTERRUPT] = true;
            break;

        case Computer::InstructionOpcode::IN: // TODO
        case Computer::InstructionOpcode::OUT: // TODO
        case Computer::InstructionOpcode::INC:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + 1) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
                cpu.m_operationCache = cpu.m_dataCache + 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_operationCache);
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_dataCache + 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case Computer::InstructionOpcode::INT: // TODO
        case Computer::InstructionOpcode::IRT:
            Cpu::pop(cpu, cpu.m_dataCache); // LSB
            cpu.m_programCounter = cpu.m_dataCache;

            Cpu::pop(cpu, cpu.m_dataCache); // MSB
            cpu.m_programCounter = (Word)cpu.m_dataCache << 8;

            Cpu::pop(cpu, cpu.m_registers[(int)Computer::Register::I]);

            cpu.m_jumpOccured = true;
            break;

        case Computer::InstructionOpcode::JMC:
            if (cpu.m_flags[(int)Computer::Flags::CARRY])
            {
                Cpu::jump(cpu);
            }
            break;

        case Computer::InstructionOpcode::JME:
            if (cpu.m_flags[(int)Computer::Flags::EQUAL])
            {
                Cpu::jump(cpu);
            }
            break;

        case Computer::InstructionOpcode::JMN:
            if (cpu.m_flags[(int)Computer::Flags::NEGATIVE])
            {
                Cpu::jump(cpu);
            }
            break;

        case Computer::InstructionOpcode::JMP:
            Cpu::jump(cpu);
            break;

        case Computer::InstructionOpcode::JMS:
            if (cpu.m_flags[(int)Computer::Flags::SUPERIOR])
            {
                Cpu::jump(cpu);
            }
            break;

        case Computer::InstructionOpcode::JMZ:
            if (cpu.m_flags[(int)Computer::Flags::ZERO])
            {
                Cpu::jump(cpu);
            }
            break;

        case Computer::InstructionOpcode::JMF:
            if (cpu.m_flags[(int)Computer::Flags::INFERIOR])
            {
                Cpu::jump(cpu);
            }
            break;

        case Computer::InstructionOpcode::STR:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_registers[(int)cpu.m_register3Index]);
            }
            break;

        case Computer::InstructionOpcode::LOD:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_registers[(int)cpu.m_register3Index] = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
            }
            break;

        case Computer::InstructionOpcode::MOV:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_registers[(int)cpu.m_register2Index];
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_v1;
            }
            break;

        case Computer::InstructionOpcode::NOT:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = ~cpu.m_registers[(int)cpu.m_register1Index];

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = ~cpu.m_dataCache;

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case Computer::InstructionOpcode::OR:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_v1;

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_dataCache;

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        case Computer::InstructionOpcode::POP:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                Cpu::pop(cpu, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            break;

        case Computer::InstructionOpcode::PSH:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                Cpu::push(cpu, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            break;
        case Computer::InstructionOpcode::RET:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
            {
                Cpu::pop(cpu, cpu.m_dataCache); // LSB
                cpu.m_programCounter = cpu.m_dataCache;

                Cpu::pop(cpu, cpu.m_dataCache); // MSB
                cpu.m_programCounter += (Word)cpu.m_dataCache << 8;

                cpu.m_jumpOccured = true;
            }
            break;

        case Computer::InstructionOpcode::SHL:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] << 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Computer::InstructionOpcode::ASR:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] >> 1;
                cpu.m_operationCache |= cpu.m_registers[(int)cpu.m_register1Index] & 0x80; // Keeps sign flag

                cpu.m_flags[(int)Computer::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x01;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Computer::InstructionOpcode::SHR:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] >> 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x01;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                // TODO update documentation because negative flag would never be set by a right shift

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Computer::InstructionOpcode::STC:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::CARRY] = true;
            break;

        case Computer::InstructionOpcode::STE:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::EQUAL] = true;
            break;

        case Computer::InstructionOpcode::STI:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::INTERRUPT] = true;
            break;

        case Computer::InstructionOpcode::STN:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = true;
            break;

        case Computer::InstructionOpcode::STS:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::SUPERIOR] = true;
            break;

        case Computer::InstructionOpcode::STZ:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::ZERO] = true;
            break;

        case Computer::InstructionOpcode::STF:
            if (cpu.m_addressingMode == Computer::AddressingMode::NONE)
                cpu.m_flags[(int)Computer::Flags::INFERIOR] = true;
            break;

        case Computer::InstructionOpcode::SUB:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index]) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_dataCache;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Computer::InstructionOpcode::SBB:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index] - 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index] - 1) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1 - 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1 - 1) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_dataCache - 1;

                cpu.m_flags[(int)Computer::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1 - 1) < 0;
                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Computer::InstructionOpcode::XOR:
            if (cpu.m_addressingMode == Computer::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_v1;

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Computer::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_dataCache;

                cpu.m_flags[(int)Computer::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Computer::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        default:
            break;
    }
}

void Cpu::jump(HbcCpu &cpu)
{
    if (cpu.m_addressingMode == Computer::AddressingMode::REG16)
    {
            cpu.m_programCounter = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
            cpu.m_jumpOccured = true;
    }
    else if (cpu.m_addressingMode == Computer::AddressingMode::IMM16)
    {
            cpu.m_programCounter = cpu.m_vX;
            cpu.m_jumpOccured = true;
    }
}

bool Cpu::pop(HbcCpu &cpu, Byte &data)
{
    if (cpu.m_stackPointer == 0x00)
    {
        cpu.m_flags[(int)Computer::Flags::CARRY] = true;

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

bool Cpu::push(HbcCpu &cpu, Byte data)
{
    if (cpu.m_stackIsFull)
    {
        cpu.m_flags[(int)Computer::Flags::CARRY] = true;

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
