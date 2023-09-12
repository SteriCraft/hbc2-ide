#include "token.h"

using namespace Token;

// === TokenItem CLASS ===
TokenItem::TokenItem(std::string str)
{   
    m_str = str;
    m_err = Token::ErrorType::NONE;

    determineType();
}

void TokenItem::setStr(std::string str)
{
    if (!str.empty())
    {
        m_str = str;

        determineType();
    }
}

void TokenItem::setAsAddress(uint16_t address)
{
    m_type = Token::TokenType::ADDRESS;
    m_address = address;

    m_value = 0;
    m_labelName = "";
    m_str = "";
}

Token::TokenType TokenItem::getType()
{
    return m_type;
}

Cpu::InstructionOpcode TokenItem::getInstructionOpcode()
{
    return m_instructionOpcode;
}

Cpu::Register TokenItem::getRegister()
{
    return m_reg;
}

Token::ConcatReg TokenItem::getConcatRegs()
{
    return m_concatReg;
}

uint8_t TokenItem::getValue()
{
    return m_value;
}

uint16_t TokenItem::getAddress()
{
    return m_address;
}

std::string TokenItem::getLabelName()
{
    return m_labelName;
}

std::string TokenItem::getVariableName()
{
    return m_str;
}

std::string TokenItem::getStr()
{
    return m_str;
}

Token::ErrorType TokenItem::getErr()
{
    return m_err;
}

std::string TokenItem::print()
{
    switch (m_type)
    {
        case Token::TokenType::INSTR:
            return "Instruction " + Cpu::instrStrArr[(int)m_instructionOpcode];

        case Token::TokenType::DEFINE:
            return "Define keyword";

        case Token::TokenType::INCLUDE:
            return "Include keyword";

        case Token::TokenType::DATA:
            return "Data keyword";

        case Token::TokenType::LABEL:
            return "Label " + m_labelName;

        case Token::TokenType::VAR:
            return "Variable " + getVariableName();

        case Token::TokenType::DECVAL:
            return "Decimal value " + std::to_string((int)m_value);

        case Token::TokenType::HEXVAL:
            return "Hexadecimal value " + std::to_string((int)m_value);

        case Token::TokenType::ADDRESS:
            return "Address " + std::to_string((int)m_address);

        case Token::TokenType::REG:
            return "Register " + Cpu::regStrArr[(int)m_reg];

        case Token::TokenType::CONCATREG:
            return "Concatenated registers [" + Cpu::regStrArr[(int)m_concatReg.msReg] + "<<" + Cpu::regStrArr[(int)m_concatReg.lsReg] + "]";

        case Token::TokenType::STRING:
            return "String \"" + m_str + "\"";

        case Token::TokenType::INVALID:
            return "Invalid TokenItem";

        default:
            return "Undefined TokenItem";
    }
}

void TokenItem::determineType()
{
    size_t colonPos = m_str.find(':');

    if (m_str.empty())
        m_type = Token::TokenType::INVALID;
    else
    {
        if (m_str == ".define")
            m_type = Token::TokenType::DEFINE;
        else if (m_str == ".include")
            m_type = Token::TokenType::INCLUDE;
        else if (m_str == ".data")
            m_type = Token::TokenType::DATA;
        else if (colonPos != std::string::npos)
        {
            if (m_str.find(':', colonPos + 1) == std::string::npos)
            {
                m_type = Token::TokenType::LABEL;
                m_labelName = m_str.substr(1);
            }
            else
            {
                m_type = Token::TokenType::INVALID;
                m_err = Token::ErrorType::INVAL_LABEL;
                return;
            }
        }
        else if (identifyInstruction())
            m_type = Token::TokenType::INSTR;
        else
            analyseArgument();
    }
}

bool TokenItem::identifyInstruction()
{
    for (unsigned int i(0); i < Cpu::INSTRUCTIONS_NB; i++)
    {
        if (m_str == Cpu::instrStrArr[i])
        {
            m_instructionOpcode = ( Cpu::InstructionOpcode)i;
            return true;
        }
    }

    return false;
}

void TokenItem::analyseArgument()
{
    if (m_str.size() >= 2)
    {
        // STRING
        if (m_str[0] == '\"' && m_str[m_str.size() - 1] == '\"')
        {
            if (m_str.size() > 2)
            {
                m_str = m_str.substr(1, m_str.size() - 2);
                m_type = Token::TokenType::STRING;
            }
            else
            {
                m_type = Token::TokenType::INVALID;
                m_err = Token::ErrorType::EMPTY_STRING;
            }

            return;
        }
    }

    if (is_validToken(m_str))
    {
        if (is_register(m_str)) // REG
        {
            for (unsigned int i(0); i < Cpu::REGISTERS_NB; i++)
            {
                if (m_str == Cpu::regStrArr[i])
                {
                    m_reg = (Cpu::Register)i;
                    m_type = Token::TokenType::REG;
                    return;
                }
            }
        }

        if (is_digits(m_str)) // DECVAL
        {
            int value = std::stoi(m_str, nullptr, 10);

            if (value < 256)
            {
                m_value = value;
                m_type = Token::TokenType::DECVAL;
            }
            else
            {
                m_type = Token::TokenType::INVALID;
                m_err = Token::ErrorType::INVAL_DECVAL_TOO_HIGH;
            }

            return;
        }

        if (m_str == "0x")
        {
            m_type = Token::TokenType::INVALID;
            m_err = Token::ErrorType::INVAL_HEXVAL;
            return;
        }

        if (m_str == "$")
        {
            m_type = Token::TokenType::INVALID;
            m_err = Token::ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str == "$0")
        {
            m_type = Token::TokenType::INVALID;
            m_err = Token::ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str == "$0x")
        {
            m_type = Token::TokenType::INVALID;
            m_err = Token::ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str.size() > 1)
        {
            if (m_str[0] == '$')
            {
                if (!is_address(m_str.substr(1)))
                {
                    m_type = Token::TokenType::INVALID;
                    m_err = Token::ErrorType::INVAL_ADDR;
                    return;
                }
            }
        }

        if (m_str.size() > 2)
        {
            if (m_str.substr(0, 2) == "0x") // HEXVAL
            {
                if (is_hexDigits(m_str.substr(2)))
                {
                    int value = std::stoi(m_str.substr(2), nullptr, 16);

                    if (value < 256)
                    {
                        m_value = value;
                        m_type = Token::TokenType::HEXVAL;
                    }
                    else
                    {
                        m_type = Token::TokenType::INVALID;
                        m_err = Token::ErrorType::INVAL_HEXVAL_TOO_HIGH;
                    }
                }
                else
                {
                    m_type = Token::TokenType::INVALID;
                    m_err = Token::ErrorType::INVAL_HEXVAL;
                }

                return;
            }
        }

        if (m_str.size() > 3)
        {
            if (m_str.substr(0, 3) == "$0x")
            {
                if (is_hexDigits(m_str.substr(3)))
                {
                    int value = std::stoi(m_str.substr(3), nullptr, 16);

                    if (value < 65536)
                    {
                        m_address = value;
                        m_type = Token::TokenType::ADDRESS;
                    }
                    else
                    {
                        m_type = Token::TokenType::INVALID;
                        m_err = Token::ErrorType::INVAL_ADDR_TOO_HIGH;
                    }
                }
                else
                {
                    m_type = Token::TokenType::INVALID;
                    m_err = Token::ErrorType::INVAL_ADDR;
                }

                return;
            }
        }

        if (m_str.find("[") != std::string::npos || m_str.find("]") != std::string::npos)
        {
            if (m_str.size() >= 2)
            {
                if (m_str[0] == '[' && m_str[m_str.size() - 1] == ']')
                {
                    if (m_str.find("[", 1) == std::string::npos && m_str.find("]") == (m_str.size() - 1))
                    {
                        if (m_str.size() == 4)
                        {
                            int msReg(-1); // Most significant byte represented by register
                            int lsReg(-1); // Least [...]

                            for (unsigned int i(0); i < Cpu::REGISTERS_NB; i++)
                            {
                                if (m_str[1] == Cpu::regStrArr[i][0])
                                {
                                    msReg = i;
                                    break;
                                }
                            }

                            for (unsigned int i(0); i < Cpu::REGISTERS_NB; i++)
                            {
                                if (m_str[2] == Cpu::regStrArr[i][0])
                                {
                                    lsReg = i;
                                    break;
                                }
                            }

                            if (msReg != -1 && lsReg != -1)
                            {
                                m_concatReg.msReg = (Cpu::Register)msReg;
                                m_concatReg.lsReg = (Cpu::Register)lsReg;
                                m_type = Token::TokenType::CONCATREG;
                            }
                            else
                            {
                                m_type = Token::TokenType::INVALID;
                                m_err = Token::ErrorType::INVAL_CONCATREG_REG;
                            }
                        }
                        else
                        {
                            m_type = Token::TokenType::INVALID;
                            m_err = Token::ErrorType::INVAL_CONCATREG_SIZE;
                        }
                    }
                    else
                    {
                        m_type = Token::TokenType::INVALID;
                        m_err = Token::ErrorType::INVAL_CONCATREG_REG;
                    }

                    return;
                }
                else
                {
                    m_type = Token::TokenType::INVALID;
                    m_err = Token::ErrorType::INVAL_CONCATREG_REG;
                }
            }
            else
            {
                m_type = Token::TokenType::INVALID;
                m_err = Token::ErrorType::INVAL_CONCATREG_REG;
            }
        }
        else
        {
            m_type = Token::TokenType::VAR;
        }
    }
    else
    {
        m_type = Token::TokenType::INVALID;
        m_err = Token::ErrorType::UNAUTHOR_CHAR;
    }
}

bool TokenItem::is_digits(const std::string& str)
{
    return str.find_first_not_of("0123456789") == std::string::npos;
}

bool TokenItem::is_hexDigits(const std::string& str)
{
    return str.find_first_not_of("0123456789abcdefABCDEF") == std::string::npos;
}

bool TokenItem::is_address(const std::string& str)
{
    if (str.size() < 3)
        return false;

    if (str.substr(0, 2) != "0x")
        return false;

    return str.substr(2).find_first_not_of("0123456789abcdefABCDEF") == std::string::npos;
}

bool TokenItem::is_register(const std::string& str)
{
    return str.find_first_not_of("abcdijxy") == std::string::npos;
}

bool TokenItem::is_validToken(const std::string& str)
{
    return str.find_first_of("?,;:/!§*µù%^¨¤£&é~\"#'{(-|è`\\ç)=+}") == std::string::npos;
}


// === TokenItem LINE CLASS ===
TokenLine::TokenLine()
{
    m_err = Token::ErrorType::NONE;
    m_additionalInfo = "";
}

bool TokenLine::checkValidity()
{
    // "define" and "include" macros are not checked because they are checked previously in assembler

    unsigned int argsNb = (unsigned int)m_tokens.size() - 1;
    Token::TokenType arg1 = Token::TokenType::INVALID, arg2 = Token::TokenType::INVALID;

    if (m_tokens.empty())
    {
        m_err = Token::ErrorType::INVAL_EXPR;
    }
    else if (m_tokens[0].getType() == Token::TokenType::INSTR)
    {
        m_instr.m_opcode = m_tokens[0].getInstructionOpcode();

        if (argsNb >= 1)
        {
            arg1 = m_tokens[1].getType();

            if (argsNb >= 2)
                arg2 = m_tokens[2].getType();
        }

        switch (m_instr.m_opcode)
        {
        case  Cpu::InstructionOpcode::NOP:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::ADC:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Token::TokenType::ADDRESS || arg2 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::ADD:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Token::TokenType::ADDRESS || arg2 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::AND:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Token::TokenType::ADDRESS || arg2 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::CAL:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::CLC:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::CLE:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::CLI:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::CLN:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::CLS:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::CLZ:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::CLF:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::CMP:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::RAMREG_IMMREG;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::DEC:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::HLT:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::IN:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG && arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::OUT:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG && arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::INC:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::INT:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::DECVAL || arg1 == Token::TokenType::HEXVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM8;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::IRT:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::JMC:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::JME:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::JMN:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::JMP:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::JMS:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::JMZ:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::JMF:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG16;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::STR:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg2 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg1 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::RAMREG_IMMREG;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::LOD:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::RAMREG_IMMREG;
            }
            else if (arg2 == Token::TokenType::ADDRESS || arg2 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::MOV:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::NOT:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg1 == Token::TokenType::ADDRESS || arg1 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::IMM16;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::OR:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Token::TokenType::ADDRESS || arg2 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::POP:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::PSH:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::RET:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::SHL:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::ASR:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::SHR:
            if (argsNb != 1)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::STC:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::STE:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::STI:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::STN:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::STS:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::STZ:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::STF:
            if (argsNb > 0)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Cpu::AddressingMode::NONE;
            break;

        case  Cpu::InstructionOpcode::SUB:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Token::TokenType::ADDRESS || arg2 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::SBB:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Token::TokenType::ADDRESS || arg2 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Cpu::InstructionOpcode::XOR:
            if (argsNb != 2)
            {
                m_err = Token::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Token::TokenType::REG)
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Token::TokenType::REG)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG;
            }
            else if (arg2 == Token::TokenType::HEXVAL || arg2 == Token::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Token::TokenType::ADDRESS || arg2 == Token::TokenType::VAR)
            {
                m_instr.m_addrMode = Cpu::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Token::ErrorType::INSTR_ARG_INVAL;
            }
            break;
        }
    }
    else if (m_tokens[0].getType() == Token::TokenType::DATA)
    {
        // [VAR] [HEX | DEC]+
        // [VAR] [STRING]
        // [VAR] [ADDR] [HEX | DEC]+
        // [VAR] [ADDR] [STRING]

        if (argsNb < 2)
        {
            m_err = Token::ErrorType::DATA_ARG_NB;
        }
        else if (m_tokens[1].getType() != Token::TokenType::VAR)
        {
            m_err = Token::ErrorType::DATA_VAR_MISS;
        }
        else if (argsNb == 2) // [VAR] [STRING | HEX | DEC]
        {
            if (m_tokens[2].getType() != Token::TokenType::DECVAL && m_tokens[2].getType() != Token::TokenType::HEXVAL
             && m_tokens[2].getType() != Token::TokenType::STRING)
            {
                m_err = Token::ErrorType::DATA_INVAL;
            }
            else if (m_tokens[2].getType() == Token::TokenType::STRING)
            {
                m_dataType = Token::DataType::STRING_UNDEFINED;
            }
            else
            {
                m_dataType = Token::DataType::SINGLE_VALUE_UNDEFINED;
            }
        }
        else
        {
            if (argsNb == 3)
            {
                if (m_tokens[2].getType() == Token::TokenType::ADDRESS)
                {
                    if (m_tokens[3].getType() == Token::TokenType::STRING) // [VAR] [ADDR] [STRING]
                    {
                        m_dataType = Token::DataType::STRING_DEFINED;
                    }
                    else if (m_tokens[3].getType() == Token::TokenType::DECVAL || m_tokens[3].getType() == Token::TokenType::HEXVAL) // [VAR] [ADDR] [DEC | HEX]
                    {
                        m_dataType = Token::DataType::SINGLE_VALUE_DEFINED;
                    }
                    else
                    {
                        m_err = Token::ErrorType::DATA_INVAL;
                    }
                }
                else if ((m_tokens[2].getType() == Token::TokenType::DECVAL || m_tokens[2].getType() == Token::TokenType::HEXVAL)
                      && (m_tokens[3].getType() == Token::TokenType::DECVAL || m_tokens[3].getType() == Token::TokenType::HEXVAL))
                {
                    m_dataType = Token::DataType::MULTIPLE_VALUES_UNDEFINED;
                }
                else
                {
                    m_err = Token::ErrorType::DATA_INVAL;
                }
            }
            else
            {
                if (m_tokens[2].getType() == Token::TokenType::ADDRESS) // [VAR] [ADDR] [DEC | HEX]+
                {
                    for (unsigned int i(3); i < m_tokens.size(); i++)
                    {
                        if (m_tokens[i].getType() != Token::TokenType::DECVAL && m_tokens[i].getType() != Token::TokenType::HEXVAL)
                        {
                            m_err = Token::ErrorType::DATA_INVAL;
                        }
                    }

                    m_dataType = Token::DataType::MULTIPLE_VALUES_DEFINED;
                }
                else // [VAR] [DEC | HEX]+
                {
                    for (unsigned int i(2); i < m_tokens.size(); i++)
                    {
                        if (m_tokens[i].getType() != Token::TokenType::DECVAL && m_tokens[i].getType() != Token::TokenType::HEXVAL)
                        {
                            m_err = Token::ErrorType::DATA_INVAL;
                        }
                    }

                    m_dataType = Token::DataType::MULTIPLE_VALUES_UNDEFINED;
                }
            }
        }
    }
    else if (m_tokens[0].getType() == Token::TokenType::LABEL)
    {
        if (argsNb > 2)
        {
            m_err = Token::ErrorType::LABEL_ARG_NB;
        }
        else if (argsNb == 1) // [ADDRESS]
        {
            if (m_tokens[1].getType() != Token::TokenType::ADDRESS)
            {
                m_err = Token::ErrorType::LABEL_INVAL_ARG;
            }
            else if (m_tokens[0].getLabelName() == "_start")
            {
                m_err = Token::ErrorType::LABEL_START_REDEFINED;
            }
        }
    }
    else
    {
        m_err = Token::ErrorType::INVAL_EXPR;
    }

    return m_err == Token::ErrorType::NONE;
}

Token::Instruction TokenLine::getInstruction()
{
    return m_instr;
}

Token::DataType TokenLine::getDataType()
{
    return m_dataType;
}

bool TokenLine::isInstruction()
{
    return m_tokens[0].getType() == Token::TokenType::INSTR;
}

bool TokenLine::isData()
{
    return m_tokens[0].getType() == Token::TokenType::DATA;
}

bool TokenLine::isLabel()
{
    return m_tokens[0].getType() == Token::TokenType::LABEL;
}
