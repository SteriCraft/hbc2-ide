
#ifndef TOKEN_H
#define TOKEN_H

#include <QString>
#include <QDebug>

#include "cpuDetails.h"

namespace Assembly
{
    enum class TokenType { INSTR, DEFINE, INCLUDE, DATA, LABEL, VAR, DECVAL, HEXVAL, ADDRESS, REG, CONCATREG, STRING, INVALID };

    enum class DataType { SINGLE_VALUE_DEFINED,   MULTIPLE_VALUES_DEFINED,   STRING_DEFINED,
                          SINGLE_VALUE_UNDEFINED, MULTIPLE_VALUES_UNDEFINED, STRING_UNDEFINED };

    struct Instruction {
        CPU::InstructionOpcode m_opcode;
        CPU::AddressingMode m_addrMode;
    };

    struct ConcatReg {
        CPU::Register msReg;
        CPU::Register lsReg;
    };

    enum class ErrorType { NONE, CIRC_DEPENDENCY, INVAL_EXPR, MISSING_TERM_CHAR, EXPECT_EXPR, INVAL_LABEL,
                           INVAL_CONCATREG_REG, INVAL_CONCATREG_SIZE, INVAL_DECVAL, INVAL_DECVAL_TOO_HIGH,
                           INVAL_HEXVAL, INVAL_HEXVAL_TOO_HIGH, UNAUTHOR_CHAR, EMPTY_STRING, INVAL_ADDR,
                           INVAL_ADDR_TOO_HIGH, INVAL_DEF, DEF_ARG_NB, DEF_VAR_MISS, DEF_ALREADY_EXIST,
                           DEF_REPLAC_ERR, FILE_NOT_FOUND, NO_MAIN_FILE, INVAL_INC, INC_ARG_NB, INC_FILE_NAME_MISS,
                           FILE_NOT_READ, LABEL_ARG_NB, LABEL_INVAL_ARG, DATA_ARG_NB, DATA_VAR_MISS, DATA_INVAL,
                           INSTR_ARG_INVAL, INSTR_ARG_NB, MEM_SIZE, MEM_USE, INSTR_ALONE, LABEL_ALREADY_USED,
                           START_ROUTINE_MISSING, LABEL_START_REDEFINED, DATA_OVERWRITES_INSTR, MEM_USE_DEF_ROUTINES,
                           DATA_MEM_USE, SPLIT_MEM, LABEL_MEM_USE, DATA_OVERLAP, ROUTINE_OVERLAP, UNKNOWN_VARIABLE,
                           BIN_FILE_OPEN };
    const std::string errStr[] = { "No error", "Circular dependency on file ", "Invalid expression or string",
                                   "Missing '\"' termination character", "Expected expression",
                                   "Unexpected ':' character in label definition", "Invalid concatenated register expression",
                                   "Too many or too few registers concatenated", "Invalid decimal integer",
                                   "Decimal value > 255", "Invalid hexadecimal integer", "Hexadecimal value > 255",
                                   "Use of unauthorized character ( ?,;:/!§*µù%^¨¤£&é~\"#'{(-|è`\\ç)=+} )",
                                   "Empty string", "Invalid hexadecimal address", "Hexadecimal address > 65535",
                                   "Invalid define macro", "Too many or too few arguments for define macro",
                                   "Variable to replace missing in define macro", "Macro already defined in file ",
                                   "Invalid token in define macro in file ", "",
                                   "\"main.has\" file missing, unable to assemble the code",
                                   "Invalid include macro", "Too many or too few arguments for include macro",
                                   "File to include missing", "One or multiple files could not be read",
                                   "Too many arguments in label definition (address only)",
                                   "Invalid argument in label definition (address only)",
                                   "Too few arguments in data definition", "Variable name missing in data definition",
                                   "Invalid values in data definition (single string, single value and list of values only)",
                                   "Invalid argument(s) for that instruction", "Too few or too many arguments for that instruction",
                                   "Program size too large (", "Exceeding memory size for instructions, try automatic address definition for variables",
                                   "Instructions must be part of routine block (defined after a label)", "This label name is already used",
                                   "\"_start\" routine is missing", "_start label cannot be redefined (address 0x300 by default)",
                                   "Data definition overlaps routine \"",
                                   "Defined routines exceed memory size", "Data definition exceeds memory size",
                                   "Error on free memory space split while analyzing defined data",
                                   "Routine exceeds memory size", "Data definition overlaps \"", "Routine overlaps \"",
                                   "Unknown variable", "Unable to write the binary file" };
}

class Token // General items containing the string represented
{
    public:
        Token(std::string str);

        void setStr(std::string str);
        void setAsAddress(uint16_t address);

        Assembly::TokenType getType();
        CPU::InstructionOpcode getInstructionOpcode();
        CPU::Register getRegister();
        Assembly::ConcatReg getConcatRegs();
        uint8_t getValue();
        uint16_t getAddress();
        std::string getLabelName();
        std::string getVariableName();

        std::string getStr();
        Assembly::ErrorType getErr();

        std::string print();

    private:
        // Methods
        void determineType();
        bool identifyInstruction();
        void analyseArgument();

        bool is_digits(const std::string& str);
        bool is_hexDigits(const std::string& str);
        bool is_address(const std::string& str);
        bool is_register(const std::string& str);
        bool is_validToken(const std::string& str);

        // Attributes
        Assembly::TokenType m_type;

        CPU::InstructionOpcode m_instructionOpcode;
        CPU::Register m_reg;
        Assembly::ConcatReg m_concatReg;
        uint8_t m_value;
        uint16_t m_address;
        std::string m_labelName;

        std::string m_str;
        Assembly::ErrorType m_err;
};

class TokenLine
{
    public:
        TokenLine();

        bool checkValidity();
        Assembly::Instruction getInstruction();
        Assembly::DataType getDataType();
        bool isInstruction();
        bool isData();
        bool isLabel();

        std::vector<Token> m_tokens;
        QString m_originFilePath;
        unsigned int m_originLineNb;
        std::string m_originStr;

        Assembly::Instruction m_instr;

        Assembly::DataType m_dataType;

        Assembly::ErrorType m_err;
        std::string m_additionalInfo;
};

struct TokenFile
{
    std::vector<TokenLine> m_lines;

    QString m_fileName;
    QString m_filePath;
};

#endif // TOKEN_H
