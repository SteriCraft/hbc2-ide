#include "motherboard.h"

void Cpu::init(HbcCpu &cpu, HbcMotherboard* mb)
{
    for (unsigned int i(0); i < REGISTERS_NB; i++)
        cpu.m_registers[i] = 0x00;

    for (unsigned int i(0); i < FLAGS_NB; i++)
        cpu.m_flags[i] = (i == (int)Cpu::Flags::INTERRUPT) ? true : false;

    cpu.m_instructionRegister = 0x00000000;

    cpu.m_jumpOccured = false;
    cpu.m_programCounter = PROGRAM_START_ADDRESS;
    cpu.m_stackPointer = 0x00;
    cpu.m_stackIsFull = false;

    cpu.m_opcode = Cpu::InstructionOpcode::NOP;
    cpu.m_addressingMode = Cpu::AddressingMode::NONE;

    cpu.m_register1Index = Cpu::Register::A;
    cpu.m_register2Index = Cpu::Register::A;
    cpu.m_register3Index = Cpu::Register::A;

    cpu.m_v1 = 0x00;
    cpu.m_v2 = 0x00;
    cpu.m_vX = 0x0000;

    cpu.m_operationCache = 0x00;
    cpu.m_dataCache = 0x00;
    cpu.m_addressCache = 0x0000;

    cpu.m_currentState = Cpu::CpuState::INSTRUCTION_EXEC;
    cpu.m_motherboard = mb;

    cpu.m_lastExecutedInstructionAddress = PROGRAM_START_ADDRESS;

    cpu.m_softwareInterrupt = false;
}

void Cpu::tick(HbcCpu &cpu)
{
    switch (cpu.m_currentState)
    {
        case Cpu::CpuState::INSTRUCTION_EXEC:
            if (cpu.m_flags[(int)Cpu::Flags::INTERRUPT])
            {
                if (cpu.m_motherboard->m_int)
                {
                    cpu.m_motherboard->m_inr = true;

                    cpu.m_currentState = Cpu::CpuState::INTERRUPT_MANAGEMENT;
                    break;
                }
                else if (cpu.m_softwareInterrupt)
                {
                    cpu.m_softwareInterrupt = false;

                    cpu.m_currentState = Cpu::CpuState::INTERRUPT_MANAGEMENT;
                    break;
                }
            }

            if (!cpu.m_flags[(int)Cpu::Flags::HALT])
            {
                Cpu::fetch(cpu);
                Cpu::decode(cpu);
                Cpu::execute(cpu);

                cpu.m_lastExecutedInstructionAddress = cpu.m_programCounter;

                if (!cpu.m_jumpOccured)
                    cpu.m_programCounter += INSTRUCTION_SIZE;
                else
                    cpu.m_jumpOccured = false;
            }
            break;

        case Cpu::CpuState::INTERRUPT_MANAGEMENT:
            if (!cpu.m_motherboard->m_int)
            {
                cpu.m_motherboard->m_inr = false;
                Cpu::push(cpu, cpu.m_registers[(int)Cpu::Register::I]);

                Cpu::push(cpu, (Byte)(cpu.m_programCounter >> 8));   // MSB
                Cpu::push(cpu, (Byte)(cpu.m_programCounter & 0x00FF)); // LSB

                cpu.m_flags[(int)Cpu::Flags::INTERRUPT] = false;
                cpu.m_flags[(int)Cpu::Flags::HALT] = false;

                Word ivtAddress(Cpu::IVT_START_ADDRESS + (cpu.m_motherboard->m_addressBus & INTERRUPT_PORT_MASK) * 2);

                cpu.m_registers[(int)Cpu::Register::I] = cpu.m_motherboard->m_dataBus;

                cpu.m_programCounter = (Word)Motherboard::readRam(*cpu.m_motherboard, ivtAddress) << 8;
                cpu.m_programCounter += Motherboard::readRam(*cpu.m_motherboard, ivtAddress + 1);

                cpu.m_currentState = Cpu::CpuState::INSTRUCTION_EXEC;
            }
            break;
    }
}

void Cpu::fetch(HbcCpu &cpu)
{
    cpu.m_instructionRegister =  ((Dword)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter))     << 24;
    cpu.m_instructionRegister += ((Dword)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 1)) << 16;
    cpu.m_instructionRegister += ((Dword)Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 2)) << 8;
    cpu.m_instructionRegister +=         Motherboard::readRam(*cpu.m_motherboard, cpu.m_programCounter + 3);
}

void Cpu::decode(HbcCpu &cpu)
{
    cpu.m_opcode = (Cpu::InstructionOpcode)((cpu.m_instructionRegister & OPCODE_MASK) >> 26);
    cpu.m_addressingMode = (Cpu::AddressingMode)((cpu.m_instructionRegister & ADDRMODE_MASK) >> 22);

    cpu.m_register1Index = (Cpu::Register)((cpu.m_instructionRegister & R1_MASK) >> 19);
    cpu.m_register2Index = (Cpu::Register)((cpu.m_instructionRegister & R2_MASK) >> 16);
    cpu.m_register3Index = (Cpu::Register)((cpu.m_instructionRegister & R3_MASK) >> 8);

    cpu.m_v1 = (cpu.m_instructionRegister & V1_MASK) >> 8;
    cpu.m_v2 = cpu.m_instructionRegister & V2_MASK;
    cpu.m_vX = cpu.m_instructionRegister & VX_MASK;
}

void Cpu::execute(HbcCpu &cpu)
{
    switch (cpu.m_opcode)
    {
        case Cpu::InstructionOpcode::ADC:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index] + 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index] + 1) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1 + 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1 + 1) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;

            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache + 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Cpu::InstructionOpcode::ADD:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_registers[(int)cpu.m_register2Index]) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_v1) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + cpu.m_dataCache) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Cpu::InstructionOpcode::AND:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] & cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] & cpu.m_v1;

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] &= cpu.m_dataCache;

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        case Cpu::InstructionOpcode::CAL:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];

                Cpu::push(cpu, (Byte)(cpu.m_programCounter >> 8));     // MSB
                Cpu::push(cpu, (Byte)(cpu.m_programCounter & 0x00FF)); // LSB

                cpu.m_programCounter = cpu.m_addressCache;

                cpu.m_jumpOccured = true;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::IMM16)
            {
                Cpu::push(cpu, (Byte)(cpu.m_programCounter >> 8));     // MSB
                Cpu::push(cpu, (Byte)(cpu.m_programCounter & 0x00FF)); // LSB

                cpu.m_programCounter = cpu.m_vX;

                cpu.m_jumpOccured = true;
            }
            break;

        case Cpu::InstructionOpcode::CLC:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::CARRY] = false;
            break;

        case Cpu::InstructionOpcode::CLE:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::EQUAL] = false;
            break;

        case Cpu::InstructionOpcode::CLI:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::INTERRUPT] = false;
            break;

        case Cpu::InstructionOpcode::CLN:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = false;
            break;

        case Cpu::InstructionOpcode::CLS:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::SUPERIOR] = false;
            break;

        case Cpu::InstructionOpcode::CLZ:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::ZERO] = false;
            break;

        case Cpu::InstructionOpcode::CLF:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::INFERIOR] = false;
            break;

        case Cpu::InstructionOpcode::CMP:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_flags[(int)Cpu::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register1Index] >  cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_flags[(int)Cpu::Flags::EQUAL] =    cpu.m_registers[(int)cpu.m_register1Index] == cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_flags[(int)Cpu::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register1Index] <  cpu.m_registers[(int)cpu.m_register2Index];
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_flags[(int)Cpu::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register1Index] >  cpu.m_v1;
                cpu.m_flags[(int)Cpu::Flags::EQUAL] =    cpu.m_registers[(int)cpu.m_register1Index] == cpu.m_v1;
                cpu.m_flags[(int)Cpu::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register1Index] <  cpu.m_v1;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);

                cpu.m_flags[(int)Cpu::Flags::SUPERIOR] = cpu.m_registers[(int)cpu.m_register3Index] >  cpu.m_dataCache;
                cpu.m_flags[(int)Cpu::Flags::EQUAL]    = cpu.m_registers[(int)cpu.m_register3Index] == cpu.m_dataCache;
                cpu.m_flags[(int)Cpu::Flags::INFERIOR] = cpu.m_registers[(int)cpu.m_register3Index] <  cpu.m_dataCache;
            }
            break;

        case Cpu::InstructionOpcode::DEC:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] - 1) < 0;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
                cpu.m_operationCache = cpu.m_dataCache - 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_dataCache - 1) < 0;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_operationCache);
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_dataCache - 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_dataCache - 1) < 0;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case Cpu::InstructionOpcode::HLT:
            cpu.m_flags[(int)Cpu::Flags::HALT] = true;
            cpu.m_flags[(int)Cpu::Flags::INTERRUPT] = true;
            break;

        case Cpu::InstructionOpcode::IN:
            cpu.m_registers[(int)cpu.m_register1Index] = Iod::getPortData(cpu.m_motherboard->m_iod, cpu.m_registers[(int)cpu.m_register2Index]);
            break;

        case Cpu::InstructionOpcode::OUT:
            Iod::setPortData(cpu.m_motherboard->m_iod, cpu.m_registers[(int)cpu.m_register1Index], cpu.m_registers[(int)cpu.m_register2Index]);
            break;

        case Cpu::InstructionOpcode::INC:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] + 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_registers[(int)cpu.m_register1Index] + 1) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG16)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
                cpu.m_operationCache = cpu.m_dataCache + 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_operationCache);
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_dataCache + 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = ((int)cpu.m_dataCache + 1) > 0xFF;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case Cpu::InstructionOpcode::INT:
            cpu.m_motherboard->m_addressBus = cpu.m_v1;
            cpu.m_motherboard->m_dataBus = cpu.m_registers[(int)Cpu::Register::I];

            cpu.m_softwareInterrupt = true;
            break;

        case Cpu::InstructionOpcode::IRT:
            Cpu::pop(cpu, cpu.m_dataCache); // LSB
            cpu.m_programCounter = cpu.m_dataCache;

            Cpu::pop(cpu, cpu.m_dataCache); // MSB
            cpu.m_programCounter += (Word)cpu.m_dataCache << 8;

            Cpu::pop(cpu, cpu.m_registers[(int)Cpu::Register::I]);
            cpu.m_flags[(int)Cpu::Flags::INTERRUPT] = true;

            cpu.m_jumpOccured = true;
            break;

        case Cpu::InstructionOpcode::JMC:
            if (cpu.m_flags[(int)Cpu::Flags::CARRY])
            {
                Cpu::jump(cpu);
            }
            break;

        case Cpu::InstructionOpcode::JME:
            if (cpu.m_flags[(int)Cpu::Flags::EQUAL])
            {
                Cpu::jump(cpu);
            }
            break;

        case Cpu::InstructionOpcode::JMN:
            if (cpu.m_flags[(int)Cpu::Flags::NEGATIVE])
            {
                Cpu::jump(cpu);
            }
            break;

        case Cpu::InstructionOpcode::JMP:
            Cpu::jump(cpu);
            break;

        case Cpu::InstructionOpcode::JMS:
            if (cpu.m_flags[(int)Cpu::Flags::SUPERIOR])
            {
                Cpu::jump(cpu);
            }
            break;

        case Cpu::InstructionOpcode::JMZ:
            if (cpu.m_flags[(int)Cpu::Flags::ZERO])
            {
                Cpu::jump(cpu);
            }
            break;

        case Cpu::InstructionOpcode::JMF:
            if (cpu.m_flags[(int)Cpu::Flags::INFERIOR])
            {
                Cpu::jump(cpu);
            }
            break;

        case Cpu::InstructionOpcode::STR:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_addressCache, cpu.m_registers[(int)cpu.m_register3Index]);
            }
            break;

        case Cpu::InstructionOpcode::LOD:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::RAMREG_IMMREG)
            {
                cpu.m_addressCache = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_registers[(int)cpu.m_register3Index] = Motherboard::readRam(*cpu.m_motherboard, cpu.m_addressCache);
            }
            break;

        case Cpu::InstructionOpcode::MOV:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_registers[(int)cpu.m_register2Index];
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_v1;
            }
            break;

        case Cpu::InstructionOpcode::NOT:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] = ~cpu.m_registers[(int)cpu.m_register1Index];

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::IMM16)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = ~cpu.m_dataCache;

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                Motherboard::writeRam(*cpu.m_motherboard, cpu.m_vX, cpu.m_operationCache);
            }
            break;

        case Cpu::InstructionOpcode::OR:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_v1;

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] |= cpu.m_dataCache;

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        case Cpu::InstructionOpcode::POP:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                Cpu::pop(cpu, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            break;

        case Cpu::InstructionOpcode::PSH:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                Cpu::push(cpu, cpu.m_registers[(int)cpu.m_register1Index]);
            }
            break;
        case Cpu::InstructionOpcode::RET:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
            {
                Cpu::pop(cpu, cpu.m_dataCache); // LSB
                cpu.m_programCounter = cpu.m_dataCache;

                Cpu::pop(cpu, cpu.m_dataCache); // MSB
                cpu.m_programCounter += (Word)cpu.m_dataCache << 8;

                cpu.m_programCounter += Cpu::INSTRUCTION_SIZE;

                cpu.m_jumpOccured = true;
            }
            break;

        case Cpu::InstructionOpcode::SHL:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] << 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Cpu::InstructionOpcode::ASR:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] >> 1;
                cpu.m_operationCache |= cpu.m_registers[(int)cpu.m_register1Index] & 0x80; // Keeps sign flag

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x01;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Cpu::InstructionOpcode::SHR:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] >> 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] & 0x01;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Cpu::InstructionOpcode::STC:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::CARRY] = true;
            break;

        case Cpu::InstructionOpcode::STE:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::EQUAL] = true;
            break;

        case Cpu::InstructionOpcode::STI:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::INTERRUPT] = true;
            break;

        case Cpu::InstructionOpcode::STN:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = true;
            break;

        case Cpu::InstructionOpcode::STS:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::SUPERIOR] = true;
            break;

        case Cpu::InstructionOpcode::STZ:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::ZERO] = true;
            break;

        case Cpu::InstructionOpcode::STF:
            if (cpu.m_addressingMode == Cpu::AddressingMode::NONE)
                cpu.m_flags[(int)Cpu::Flags::INFERIOR] = true;
            break;

        case Cpu::InstructionOpcode::SUB:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] < cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] < cpu.m_v1;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_dataCache;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] < cpu.m_dataCache;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Cpu::InstructionOpcode::SBB:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_registers[(int)cpu.m_register2Index] - 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] <= cpu.m_registers[(int)cpu.m_register2Index];
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_v1 - 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] <= cpu.m_v1;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_operationCache = cpu.m_registers[(int)cpu.m_register1Index] - cpu.m_dataCache - 1;

                cpu.m_flags[(int)Cpu::Flags::CARRY] = cpu.m_registers[(int)cpu.m_register1Index] <= cpu.m_dataCache;
                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_operationCache == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_operationCache & 0x80;

                cpu.m_registers[(int)cpu.m_register1Index] = cpu.m_operationCache;
            }
            break;

        case Cpu::InstructionOpcode::XOR:
            if (cpu.m_addressingMode == Cpu::AddressingMode::REG)
            {
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_registers[(int)cpu.m_register2Index];

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_IMM8)
            {
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_v1;

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            else if (cpu.m_addressingMode == Cpu::AddressingMode::REG_RAM)
            {
                cpu.m_dataCache = Motherboard::readRam(*cpu.m_motherboard, cpu.m_vX);
                cpu.m_registers[(int)cpu.m_register1Index] ^= cpu.m_dataCache;

                cpu.m_flags[(int)Cpu::Flags::ZERO] = cpu.m_registers[(int)cpu.m_register1Index] == 0x00;
                cpu.m_flags[(int)Cpu::Flags::NEGATIVE] = cpu.m_registers[(int)cpu.m_register1Index] & 0x80;
            }
            break;

        default:
            break;
    }
}

void Cpu::jump(HbcCpu &cpu)
{
    if (cpu.m_addressingMode == Cpu::AddressingMode::REG16)
    {
            cpu.m_programCounter = ((Word)cpu.m_registers[(int)cpu.m_register1Index] << 8) + cpu.m_registers[(int)cpu.m_register2Index];
            cpu.m_jumpOccured = true;
    }
    else if (cpu.m_addressingMode == Cpu::AddressingMode::IMM16)
    {
            cpu.m_programCounter = cpu.m_vX;
            cpu.m_jumpOccured = true;
    }
}

void Cpu::pop(HbcCpu &cpu, Byte &data)
{
    if (cpu.m_stackPointer != 0x00)
    {
        cpu.m_stackPointer--;
        cpu.m_stackIsFull = false;

        data = Motherboard::readRam(*cpu.m_motherboard, cpu.m_stackPointer);
    }
}

void Cpu::push(HbcCpu &cpu, Byte data)
{
    if (!cpu.m_stackIsFull)
    {
        Motherboard::writeRam(*cpu.m_motherboard, cpu.m_stackPointer, data);

        cpu.m_stackPointer++;
        cpu.m_stackIsFull = cpu.m_stackPointer == 0x00;
    }
}
