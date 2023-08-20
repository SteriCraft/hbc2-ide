#include "token.h"

// === TOKEN CLASS ===
Token::Token(std::string str)
{   
    m_str = str;
    m_err = Assembly::ErrorType::NONE;

    determineType();
}

void Token::setStr(std::string str)
{
    if (!str.empty())
    {
        m_str = str;

        determineType();
    }
}

void Token::setAsAddress(uint16_t address)
{
    m_type = Assembly::TokenType::ADDRESS;
    m_address = address;

    m_value = 0;
    m_labelName = "";
    m_str = "";
}

Assembly::TokenType Token::getType()
{
    return m_type;
}

Computer::InstructionOpcode Token::getInstructionOpcode()
{
    return m_instructionOpcode;
}

Computer::Register Token::getRegister()
{
    return m_reg;
}

Assembly::ConcatReg Token::getConcatRegs()
{
    return m_concatReg;
}

uint8_t Token::getValue()
{
    return m_value;
}

uint16_t Token::getAddress()
{
    return m_address;
}

std::string Token::getLabelName()
{
    return m_labelName;
}

std::string Token::getVariableName()
{
    return m_str;
}

std::string Token::getStr()
{
    return m_str;
}

Assembly::ErrorType Token::getErr()
{
    return m_err;
}

std::string Token::print()
{
    switch (m_type)
    {
        case Assembly::TokenType::INSTR:
            return "Instruction " + Computer::instrStrArr[(int)m_instructionOpcode];

        case Assembly::TokenType::DEFINE:
            return "Define keyword";

        case Assembly::TokenType::INCLUDE:
            return "Include keyword";

        case Assembly::TokenType::DATA:
            return "Data keyword";

        case Assembly::TokenType::LABEL:
            return "Label " + m_labelName;

        case Assembly::TokenType::VAR:
            return "Variable " + getVariableName();

        case Assembly::TokenType::DECVAL:
            return "Decimal value " + std::to_string((int)m_value);

        case Assembly::TokenType::HEXVAL:
            return "Hexadecimal value " + std::to_string((int)m_value);

        case Assembly::TokenType::ADDRESS:
            return "Address " + std::to_string((int)m_address);

        case Assembly::TokenType::REG:
            return "Register " + Computer::regStrArr[(int)m_reg];

        case Assembly::TokenType::CONCATREG:
            return "Concatenated registers [" + Computer::regStrArr[(int)m_concatReg.msReg] + "<<" + Computer::regStrArr[(int)m_concatReg.lsReg] + "]";

        case Assembly::TokenType::STRING:
            return "String \"" + m_str + "\"";

        case Assembly::TokenType::INVALID:
            return "Invalid token";

        default:
            return "Undefined token";
    }
}

void Token::determineType()
{
    size_t colonPos = m_str.find(':');

    if (m_str.empty())
        m_type = Assembly::TokenType::INVALID;
    else
    {
        if (m_str == ".define")
            m_type = Assembly::TokenType::DEFINE;
        else if (m_str == ".include")
            m_type = Assembly::TokenType::INCLUDE;
        else if (m_str == ".data")
            m_type = Assembly::TokenType::DATA;
        else if (colonPos != std::string::npos)
        {
            if (m_str.find(':', colonPos + 1) == std::string::npos)
            {
                m_type = Assembly::TokenType::LABEL;
                m_labelName = m_str.substr(1);
            }
            else
            {
                m_type = Assembly::TokenType::INVALID;
                m_err = Assembly::ErrorType::INVAL_LABEL;
                return;
            }
        }
        else if (identifyInstruction())
            m_type = Assembly::TokenType::INSTR;
        else
            analyseArgument();
    }
}

bool Token::identifyInstruction()
{
    for (unsigned int i(0); i < INSTRUCTIONS_NB; i++)
    {
        if (m_str == Computer::instrStrArr[i])
        {
            m_instructionOpcode = ( Computer::InstructionOpcode)i;
            return true;
        }
    }

    return false;
}

void Token::analyseArgument()
{
    if (m_str.size() >= 2)
    {
        // STRING
        if (m_str[0] == '\"' && m_str[m_str.size() - 1] == '\"')
        {
            if (m_str.size() > 2)
            {
                m_str = m_str.substr(1, m_str.size() - 2);
                m_type = Assembly::TokenType::STRING;
            }
            else
            {
                m_type = Assembly::TokenType::INVALID;
                m_err = Assembly::ErrorType::EMPTY_STRING;
            }

            return;
        }
    }

    if (is_validToken(m_str))
    {
        if (is_register(m_str)) // REG
        {
            for (unsigned int i(0); i < REGISTERS_NB; i++)
            {
                if (m_str == Computer::regStrArr[i])
                {
                    m_reg = (Computer::Register)i;
                    m_type = Assembly::TokenType::REG;
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
                m_type = Assembly::TokenType::DECVAL;
            }
            else
            {
                m_type = Assembly::TokenType::INVALID;
                m_err = Assembly::ErrorType::INVAL_DECVAL_TOO_HIGH;
            }

            return;
        }

        if (m_str == "0x")
        {
            m_type = Assembly::TokenType::INVALID;
            m_err = Assembly::ErrorType::INVAL_HEXVAL;
            return;
        }

        if (m_str == "$")
        {
            m_type = Assembly::TokenType::INVALID;
            m_err = Assembly::ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str == "$0")
        {
            m_type = Assembly::TokenType::INVALID;
            m_err = Assembly::ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str == "$0x")
        {
            m_type = Assembly::TokenType::INVALID;
            m_err = Assembly::ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str.size() > 1)
        {
            if (m_str[0] == '$')
            {
                if (!is_address(m_str.substr(1)))
                {
                    m_type = Assembly::TokenType::INVALID;
                    m_err = Assembly::ErrorType::INVAL_ADDR;
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
                        m_type = Assembly::TokenType::HEXVAL;
                    }
                    else
                    {
                        m_type = Assembly::TokenType::INVALID;
                        m_err = Assembly::ErrorType::INVAL_HEXVAL_TOO_HIGH;
                    }
                }
                else
                {
                    m_type = Assembly::TokenType::INVALID;
                    m_err = Assembly::ErrorType::INVAL_HEXVAL;
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
                        m_type = Assembly::TokenType::ADDRESS;
                    }
                    else
                    {
                        m_type = Assembly::TokenType::INVALID;
                        m_err = Assembly::ErrorType::INVAL_ADDR_TOO_HIGH;
                    }
                }
                else
                {
                    m_type = Assembly::TokenType::INVALID;
                    m_err = Assembly::ErrorType::INVAL_ADDR;
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

                            for (unsigned int i(0); i < REGISTERS_NB; i++)
                            {
                                if (m_str[1] == Computer::regStrArr[i][0])
                                {
                                    msReg = i;
                                    break;
                                }
                            }

                            for (unsigned int i(0); i < REGISTERS_NB; i++)
                            {
                                if (m_str[2] == Computer::regStrArr[i][0])
                                {
                                    lsReg = i;
                                    break;
                                }
                            }

                            if (msReg != -1 && lsReg != -1)
                            {
                                m_concatReg.msReg = (Computer::Register)msReg;
                                m_concatReg.lsReg = (Computer::Register)lsReg;
                                m_type = Assembly::TokenType::CONCATREG;
                            }
                            else
                            {
                                m_type = Assembly::TokenType::INVALID;
                                m_err = Assembly::ErrorType::INVAL_CONCATREG_REG;
                            }
                        }
                        else
                        {
                            m_type = Assembly::TokenType::INVALID;
                            m_err = Assembly::ErrorType::INVAL_CONCATREG_SIZE;
                        }
                    }
                    else
                    {
                        m_type = Assembly::TokenType::INVALID;
                        m_err = Assembly::ErrorType::INVAL_CONCATREG_REG;
                    }

                    return;
                }
                else
                {
                    m_type = Assembly::TokenType::INVALID;
                    m_err = Assembly::ErrorType::INVAL_CONCATREG_REG;
                }
            }
            else
            {
                m_type = Assembly::TokenType::INVALID;
                m_err = Assembly::ErrorType::INVAL_CONCATREG_REG;
            }
        }
        else
        {
            m_type = Assembly::TokenType::VAR;
        }
    }
    else
    {
        m_type = Assembly::TokenType::INVALID;
        m_err = Assembly::ErrorType::UNAUTHOR_CHAR;
    }
}

bool Token::is_digits(const std::string& str)
{
    return str.find_first_not_of("0123456789") == std::string::npos;
}

bool Token::is_hexDigits(const std::string& str)
{
    return str.find_first_not_of("0123456789abcdefABCDEF") == std::string::npos;
}

bool Token::is_address(const std::string& str)
{
    if (str.size() < 3)
        return false;

    if (str.substr(0, 2) != "0x")
        return false;

    return str.substr(2).find_first_not_of("0123456789abcdefABCDEF") == std::string::npos;
}

bool Token::is_register(const std::string& str)
{
    return str.find_first_not_of("abcdijxy") == std::string::npos;
}

bool Token::is_validToken(const std::string& str)
{
    return str.find_first_of("?,;:/!§*µù%^¨¤£&é~\"#'{(-|è`\\ç)=+}") == std::string::npos;
}


// === TOKEN LINE CLASS ===
TokenLine::TokenLine()
{
    m_err = Assembly::ErrorType::NONE;
    m_additionalInfo = "";
}

bool TokenLine::checkValidity()
{
    // "define" and "include" macros are not checked because they are checked previously in assembler

    unsigned int argsNb = (unsigned int)m_tokens.size() - 1;
    Assembly::TokenType arg1 = Assembly::TokenType::INVALID, arg2 = Assembly::TokenType::INVALID;

    if (m_tokens.empty())
    {
        m_err = Assembly::ErrorType::INVAL_EXPR;
    }
    else if (m_tokens[0].getType() == Assembly::TokenType::INSTR)
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
        case  Computer::InstructionOpcode::NOP:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::ADC:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Assembly::TokenType::ADDRESS || arg2 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::ADD:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Assembly::TokenType::ADDRESS || arg2 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::AND:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Assembly::TokenType::ADDRESS || arg2 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::CAL:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::CLC:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::CLE:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::CLI:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::CLN:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::CLS:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::CLZ:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::CLF:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::CMP:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::RAMREG_IMMREG;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::DEC:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::HLT:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::IN:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG && arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::OUT:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG && arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::INC:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::INT:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::DECVAL || arg1 == Assembly::TokenType::HEXVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM8;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::IRT:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::JMC:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::JME:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::JMN:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::JMP:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::JMS:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::JMZ:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::JMF:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG16;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::STR:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg2 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg1 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::RAMREG_IMMREG;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::LOD:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::CONCATREG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::RAMREG_IMMREG;
            }
            else if (arg2 == Assembly::TokenType::ADDRESS || arg2 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::MOV:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::NOT:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg1 == Assembly::TokenType::ADDRESS || arg1 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::IMM16;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::OR:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Assembly::TokenType::ADDRESS || arg2 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::POP:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::PSH:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::RET:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::SHL:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::ASR:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::SHR:
            if (argsNb != 1)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::STC:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::STE:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::STI:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::STN:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::STS:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::STZ:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::STF:
            if (argsNb > 0)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = Computer::AddressingMode::NONE;
            break;

        case  Computer::InstructionOpcode::SUB:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Assembly::TokenType::ADDRESS || arg2 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::SBB:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Assembly::TokenType::ADDRESS || arg2 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case  Computer::InstructionOpcode::XOR:
            if (argsNb != 2)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != Assembly::TokenType::REG)
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == Assembly::TokenType::REG)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG;
            }
            else if (arg2 == Assembly::TokenType::HEXVAL || arg2 == Assembly::TokenType::DECVAL)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_IMM8;
            }
            else if (arg2 == Assembly::TokenType::ADDRESS || arg2 == Assembly::TokenType::VAR)
            {
                m_instr.m_addrMode = Computer::AddressingMode::REG_RAM;
            }
            else
            {
                m_err = Assembly::ErrorType::INSTR_ARG_INVAL;
            }
            break;
        }
    }
    else if (m_tokens[0].getType() == Assembly::TokenType::DATA)
    {
        // [VAR] [HEX | DEC]+
        // [VAR] [STRING]
        // [VAR] [ADDR] [HEX | DEC]+
        // [VAR] [ADDR] [STRING]

        if (argsNb < 2)
        {
            m_err = Assembly::ErrorType::DATA_ARG_NB;
        }
        else if (m_tokens[1].getType() != Assembly::TokenType::VAR)
        {
            m_err = Assembly::ErrorType::DATA_VAR_MISS;
        }
        else if (argsNb == 2) // [VAR] [STRING | HEX | DEC]
        {
            if (m_tokens[2].getType() != Assembly::TokenType::DECVAL && m_tokens[2].getType() != Assembly::TokenType::HEXVAL
             && m_tokens[2].getType() != Assembly::TokenType::STRING)
            {
                m_err = Assembly::ErrorType::DATA_INVAL;
            }
            else if (m_tokens[2].getType() == Assembly::TokenType::STRING)
            {
                m_dataType = Assembly::DataType::STRING_UNDEFINED;
            }
            else
            {
                m_dataType = Assembly::DataType::SINGLE_VALUE_UNDEFINED;
            }
        }
        else
        {
            if (argsNb == 3)
            {
                if (m_tokens[2].getType() == Assembly::TokenType::ADDRESS)
                {
                    if (m_tokens[3].getType() == Assembly::TokenType::STRING) // [VAR] [ADDR] [STRING]
                    {
                        m_dataType = Assembly::DataType::STRING_DEFINED;
                    }
                    else if (m_tokens[3].getType() == Assembly::TokenType::DECVAL || m_tokens[3].getType() == Assembly::TokenType::HEXVAL) // [VAR] [ADDR] [DEC | HEX]
                    {
                        m_dataType = Assembly::DataType::SINGLE_VALUE_DEFINED;
                    }
                    else
                    {
                        m_err = Assembly::ErrorType::DATA_INVAL;
                    }
                }
                else if ((m_tokens[2].getType() == Assembly::TokenType::DECVAL || m_tokens[2].getType() == Assembly::TokenType::HEXVAL)
                      && (m_tokens[3].getType() == Assembly::TokenType::DECVAL || m_tokens[3].getType() == Assembly::TokenType::HEXVAL))
                {
                    m_dataType = Assembly::DataType::MULTIPLE_VALUES_UNDEFINED;
                }
                else
                {
                    m_err = Assembly::ErrorType::DATA_INVAL;
                }
            }
            else
            {
                if (m_tokens[2].getType() == Assembly::TokenType::ADDRESS) // [VAR] [ADDR] [DEC | HEX]+
                {
                    for (unsigned int i(3); i < m_tokens.size(); i++)
                    {
                        if (m_tokens[i].getType() != Assembly::TokenType::DECVAL && m_tokens[i].getType() != Assembly::TokenType::HEXVAL)
                        {
                            m_err = Assembly::ErrorType::DATA_INVAL;
                        }
                    }

                    m_dataType = Assembly::DataType::MULTIPLE_VALUES_DEFINED;
                }
                else // [VAR] [DEC | HEX]+
                {
                    for (unsigned int i(2); i < m_tokens.size(); i++)
                    {
                        if (m_tokens[i].getType() != Assembly::TokenType::DECVAL && m_tokens[i].getType() != Assembly::TokenType::HEXVAL)
                        {
                            m_err = Assembly::ErrorType::DATA_INVAL;
                        }
                    }

                    m_dataType = Assembly::DataType::MULTIPLE_VALUES_UNDEFINED;
                }
            }
        }
    }
    else if (m_tokens[0].getType() == Assembly::TokenType::LABEL)
    {
        if (argsNb > 2)
        {
            m_err = Assembly::ErrorType::LABEL_ARG_NB;
        }
        else if (argsNb == 1) // [ADDRESS]
        {
            if (m_tokens[1].getType() != Assembly::TokenType::ADDRESS)
            {
                m_err = Assembly::ErrorType::LABEL_INVAL_ARG;
            }
            else if (m_tokens[0].getLabelName() == "_start")
            {
                m_err = Assembly::ErrorType::LABEL_START_REDEFINED;
            }
        }
    }
    else
    {
        m_err = Assembly::ErrorType::INVAL_EXPR;
    }

    return m_err == Assembly::ErrorType::NONE;
}

Assembly::Instruction TokenLine::getInstruction()
{
    return m_instr;
}

Assembly::DataType TokenLine::getDataType()
{
    return m_dataType;
}

bool TokenLine::isInstruction()
{
    return m_tokens[0].getType() == Assembly::TokenType::INSTR;
}

bool TokenLine::isData()
{
    return m_tokens[0].getType() == Assembly::TokenType::DATA;
}

bool TokenLine::isLabel()
{
    return m_tokens[0].getType() == Assembly::TokenType::LABEL;
}
