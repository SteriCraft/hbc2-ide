#include "token.h"

// === TOKEN CLASS ===
Token::Token(std::string str)
{   
    m_str = str;
    m_err = ErrorType::NONE;

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
    m_type = TokenType::ADDRESS;
    m_address = address;

    m_value = 0;
    m_labelName = "";
    m_str = "";
    // TODO: m_address to update?
}

TokenType Token::getType()
{
    return m_type;
}

InstrOpcode Token::getInstructionOpcode()
{
    return m_instructionOpcode;
}

Register Token::getRegister()
{
    return m_reg;
}

ConcatReg Token::getConcatRegs()
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

ErrorType Token::getErr()
{
    return m_err;
}

std::string Token::print()
{
    switch (m_type)
    {
        case TokenType::INSTR:
            return "Instruction " + instrArr[(int)m_instructionOpcode];

        case TokenType::DEFINE:
            return "Define keyword";

        case TokenType::INCLUDE:
            return "Include keyword";

        case TokenType::DATA:
            return "Data keyword";

        case TokenType::LABEL:
            return "Label " + m_labelName;

        case TokenType::VAR:
            return "Variable " + getVariableName();

        case TokenType::DECVAL:
            return "Decimal value " + std::to_string((int)m_value);

        case TokenType::HEXVAL:
            return "Hexadecimal value " + std::to_string((int)m_value);

        case TokenType::ADDRESS:
            return "Address " + std::to_string((int)m_address);

        case TokenType::REG:
            return "Register " + regArr[(int)m_reg];

        case TokenType::CONCATREG:
            return "Concatenated registers [" + regArr[(int)m_concatReg.msReg] + "<<" + regArr[(int)m_concatReg.lsReg] + "]";

        case TokenType::STRING:
            return "String \"" + m_str + "\"";

        case TokenType::INVALID:
            return "Invalid token";

        default:
            return "Undefined token";
    }
}

void Token::determineType()
{
    size_t colonPos = m_str.find(':');

    if (m_str.empty())
        m_type = TokenType::INVALID;
    else
    {
        if (m_str == ".define")
            m_type = TokenType::DEFINE;
        else if (m_str == ".include")
            m_type = TokenType::INCLUDE;
        else if (m_str == ".data")
            m_type = TokenType::DATA;
        else if (colonPos != std::string::npos)
        {
            if (m_str.find(':', colonPos + 1) == std::string::npos)
            {
                m_type = TokenType::LABEL;
                m_labelName = m_str.substr(1);
            }
            else
            {
                m_type = TokenType::INVALID;
                m_err = ErrorType::INVAL_LABEL;
                return;
            }
        }
        else if (identifyInstruction())
            m_type = TokenType::INSTR;
        else
            analyseArgument();
    }
}

bool Token::identifyInstruction()
{
    for (unsigned int i(0); i < INSTR_NB; i++)
    {
        if (m_str == instrArr[i])
        {
            m_instructionOpcode = (InstrOpcode)i;
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
                m_type = TokenType::STRING;
            }
            else
            {
                m_type = TokenType::INVALID;
                m_err = ErrorType::EMPTY_STRING;
            }

            return;
        }
    }

    if (is_validToken(m_str))
    {
        if (is_register(m_str)) // REG
        {
            for (unsigned int i(0); i < REG_NB; i++)
            {
                if (m_str == regArr[i])
                {
                    m_reg = (Register)i;
                    m_type = TokenType::REG;
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
                m_type = TokenType::DECVAL;
            }
            else
            {
                m_type = TokenType::INVALID;
                m_err = ErrorType::INVAL_DECVAL_TOO_HIGH;
            }

            return;
        }

        if (m_str == "0x")
        {
            m_type = TokenType::INVALID;
            m_err = ErrorType::INVAL_HEXVAL;
            return;
        }

        if (m_str == "$")
        {
            m_type = TokenType::INVALID;
            m_err = ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str == "$0")
        {
            m_type = TokenType::INVALID;
            m_err = ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str == "$0x")
        {
            m_type = TokenType::INVALID;
            m_err = ErrorType::INVAL_ADDR;
            return;
        }

        if (m_str.size() > 1)
        {
            if (m_str[0] == '$')
            {
                if (!is_address(m_str.substr(1)))
                {
                    m_type = TokenType::INVALID;
                    m_err = ErrorType::INVAL_ADDR;
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
                        m_type = TokenType::HEXVAL;
                    }
                    else
                    {
                        m_type = TokenType::INVALID;
                        m_err = ErrorType::INVAL_HEXVAL_TOO_HIGH;
                    }
                }
                else
                {
                    m_type = TokenType::INVALID;
                    m_err = ErrorType::INVAL_HEXVAL;
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
                        m_type = TokenType::ADDRESS;
                    }
                    else
                    {
                        m_type = TokenType::INVALID;
                        m_err = ErrorType::INVAL_ADDR_TOO_HIGH;
                    }
                }
                else
                {
                    m_type = TokenType::INVALID;
                    m_err = ErrorType::INVAL_ADDR;
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

                            for (unsigned int i(0); i < REG_NB; i++)
                            {
                                if (m_str[1] == regArr[i][0])
                                {
                                    msReg = i;
                                    break;
                                }
                            }

                            for (unsigned int i(0); i < REG_NB; i++)
                            {
                                if (m_str[2] == regArr[i][0])
                                {
                                    lsReg = i;
                                    break;
                                }
                            }

                            if (msReg != -1 && lsReg != -1)
                            {
                                m_concatReg.msReg = (Register)msReg;
                                m_concatReg.lsReg = (Register)lsReg;
                                m_type = TokenType::CONCATREG;
                            }
                            else
                            {
                                m_type = TokenType::INVALID;
                                m_err = ErrorType::INVAL_CONCATREG_REG;
                            }
                        }
                        else
                        {
                            m_type = TokenType::INVALID;
                            m_err = ErrorType::INVAL_CONCATREG_SIZE;
                        }
                    }
                    else
                    {
                        m_type = TokenType::INVALID;
                        m_err = ErrorType::INVAL_CONCATREG_REG;
                    }

                    return;
                }
                else
                {
                    m_type = TokenType::INVALID;
                    m_err = ErrorType::INVAL_CONCATREG_REG;
                }
            }
            else
            {
                m_type = TokenType::INVALID;
                m_err = ErrorType::INVAL_CONCATREG_REG;
            }
        }
        else
        {
            m_type = TokenType::VAR;
        }
    }
    else
    {
        m_type = TokenType::INVALID;
        m_err = ErrorType::UNAUTHOR_CHAR;
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
    m_err = ErrorType::NONE;
    m_additionalInfo = "";
}

bool TokenLine::checkValidity()
{
    // "define" and "include" macros are not checked because they are checked previously in assembler

    unsigned int argsNb = (unsigned int)m_tokens.size() - 1;
    TokenType arg1 = TokenType::INVALID, arg2 = TokenType::INVALID;

    if (m_tokens.empty())
    {
        m_err = ErrorType::INVAL_EXPR;
    }
    else if (m_tokens[0].getType() == TokenType::INSTR)
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
        case InstrOpcode::NOP:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::ADC:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else if (arg2 == TokenType::ADDRESS || arg2 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::ADD:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else if (arg2 == TokenType::ADDRESS || arg2 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::AND:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else if (arg2 == TokenType::ADDRESS || arg2 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::CAL:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::CLC:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::CLE:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::CLI:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::CLN:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::CLS:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::CLZ:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::CLF:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::CMP:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else if (arg2 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::RAMREG_IMMREG;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::DEC:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::HLT:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::IN:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG && arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::OUT:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG && arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::INC:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::INT:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::DECVAL || arg1 == TokenType::HEXVAL)
            {
                m_instr.m_addrMode = AddrMode::IMM8;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::IRT:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::JMC:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::JME:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::JMN:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::JMP:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::JMS:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::JMZ:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::JMF:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::REG16;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::STR:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg2 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg1 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::RAMREG_IMMREG;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::LOD:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::CONCATREG)
            {
                m_instr.m_addrMode = AddrMode::RAMREG_IMMREG;
            }
            else if (arg2 == TokenType::ADDRESS || arg2 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::MOV:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::NOT:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg1 == TokenType::ADDRESS || arg1 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::IMM16;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::OR:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else if (arg2 == TokenType::ADDRESS || arg2 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::POP:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::PSH:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::RET:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::SHL:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::ASR:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::SHR:
            if (argsNb != 1)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::STC:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::STE:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::STI:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::STN:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::STS:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::STZ:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::STF:
            if (argsNb > 0)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }

            m_instr.m_addrMode = AddrMode::NONE;
            break;

        case InstrOpcode::SUB:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else if (arg2 == TokenType::ADDRESS || arg2 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::SBB:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else if (arg2 == TokenType::ADDRESS || arg2 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;

        case InstrOpcode::XOR:
            if (argsNb != 2)
            {
                m_err = ErrorType::INSTR_ARG_NB;
            }
            else if (arg1 != TokenType::REG)
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            else if (arg2 == TokenType::REG)
            {
                m_instr.m_addrMode = AddrMode::REG;
            }
            else if (arg2 == TokenType::HEXVAL || arg2 == TokenType::DECVAL)
            {
                m_instr.m_addrMode = AddrMode::REG_IMM8;
            }
            else if (arg2 == TokenType::ADDRESS || arg2 == TokenType::VAR)
            {
                m_instr.m_addrMode = AddrMode::REG_RAM;
            }
            else
            {
                m_err = ErrorType::INSTR_ARG_INVAL;
            }
            break;
        }
    }
    else if (m_tokens[0].getType() == TokenType::DATA)
    {
        // [VAR] [HEX | DEC]+
        // [VAR] [STRING]
        // [VAR] [ADDR] [HEX | DEC]+
        // [VAR] [ADDR] [STRING]

        if (argsNb < 2)
        {
            m_err = ErrorType::DATA_ARG_NB;
        }
        else if (m_tokens[1].getType() != TokenType::VAR)
        {
            m_err = ErrorType::DATA_VAR_MISS;
        }
        else if (argsNb == 2) // [VAR] [STRING | HEX | DEC]
        {
            if (m_tokens[2].getType() != TokenType::DECVAL && m_tokens[2].getType() != TokenType::HEXVAL
             && m_tokens[2].getType() != TokenType::STRING)
            {
                m_err = ErrorType::DATA_INVAL;
            }
            else if (m_tokens[2].getType() == TokenType::STRING)
            {
                m_dataType = DataType::STRING_UNDEFINED;
            }
            else
            {
                m_dataType = DataType::SINGLE_VALUE_UNDEFINED;
            }
        }
        else
        {
            if (argsNb == 3)
            {
                if (m_tokens[2].getType() == TokenType::ADDRESS)
                {
                    if (m_tokens[3].getType() == TokenType::STRING) // [VAR] [ADDR] [STRING]
                    {
                        m_dataType = DataType::STRING_DEFINED;
                    }
                    else if (m_tokens[3].getType() == TokenType::DECVAL || m_tokens[3].getType() == TokenType::HEXVAL) // [VAR] [ADDR] [DEC | HEX]
                    {
                        m_dataType = DataType::SINGLE_VALUE_DEFINED;
                    }
                    else
                    {
                        m_err = ErrorType::DATA_INVAL;
                    }
                }
                else if ((m_tokens[2].getType() == TokenType::DECVAL || m_tokens[2].getType() == TokenType::HEXVAL)
                      && (m_tokens[3].getType() == TokenType::DECVAL || m_tokens[3].getType() == TokenType::HEXVAL))
                {
                    m_dataType = DataType::MULTIPLE_VALUES_UNDEFINED;
                }
                else
                {
                    m_err = ErrorType::DATA_INVAL;
                }
            }
            else
            {
                if (m_tokens[2].getType() == TokenType::ADDRESS) // [VAR] [ADDR] [DEC | HEX]+
                {
                    for (unsigned int i(3); i < m_tokens.size(); i++)
                    {
                        if (m_tokens[i].getType() != TokenType::DECVAL && m_tokens[i].getType() != TokenType::HEXVAL)
                        {
                            m_err = ErrorType::DATA_INVAL;
                        }
                    }

                    m_dataType = DataType::MULTIPLE_VALUES_DEFINED;
                }
                else // [VAR] [DEC | HEX]+
                {
                    for (unsigned int i(2); i < m_tokens.size(); i++)
                    {
                        if (m_tokens[i].getType() != TokenType::DECVAL && m_tokens[i].getType() != TokenType::HEXVAL)
                        {
                            m_err = ErrorType::DATA_INVAL;
                        }
                    }

                    m_dataType = DataType::MULTIPLE_VALUES_UNDEFINED;
                }
            }
        }
    }
    else if (m_tokens[0].getType() == TokenType::LABEL)
    {
        if (argsNb > 2)
        {
            m_err = ErrorType::LABEL_ARG_NB;
        }
        else if (argsNb == 1) // [ADDRESS]
        {
            if (m_tokens[1].getType() != TokenType::ADDRESS)
            {
                m_err = ErrorType::LABEL_INVAL_ARG;
            }
            else if (m_tokens[0].getLabelName() == "_start")
            {
                m_err = ErrorType::LABEL_START_REDEFINED;
            }
        }
    }
    else
    {
        m_err = ErrorType::INVAL_EXPR;
    }

    return m_err == ErrorType::NONE;
}

Instruction TokenLine::getInstruction()
{
    return m_instr;
}

DataType TokenLine::getDataType()
{
    return m_dataType;
}

bool TokenLine::isInstruction()
{
    return m_tokens[0].getType() == TokenType::INSTR;
}

bool TokenLine::isData()
{
    return m_tokens[0].getType() == TokenType::DATA;
}

bool TokenLine::isLabel()
{
    return m_tokens[0].getType() == TokenType::LABEL;
}
