#ifndef TOKEN_H
#define TOKEN_H

/*!
 * \file token.h
 * \brief Defines token elements used in the assembly
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QString>
#include <QDebug>
#include "computerDetails.h"

/*!
 * \namespace Token
 * \brief Defines token elements used in the assembly
 */
namespace Token
{
    /*!
     * \enum TokenType
     * \brief Lists token types
     */
    enum class TokenType { INSTR, DEFINE, INCLUDE, DATA, LABEL, VAR, DECVAL, HEXVAL, ADDRESS, EEPROM_ADDRESS, REG, CONCATREG, STRING, INVALID };

    /*!
     * \enum DataType
     * \brief Lists data types (more info in the assembly language documentation)
     */
    enum class DataType { SINGLE_VALUE_DEFINED,   MULTIPLE_VALUES_DEFINED,   STRING_DEFINED,
                          SINGLE_VALUE_UNDEFINED, MULTIPLE_VALUES_UNDEFINED, STRING_UNDEFINED };

    /*!
     * \struct Instruction
     * \brief Opcode and addressing mode
     */
    struct Instruction {
        Cpu::InstructionOpcode m_opcode; //!< More info in the CPU documentation
        Cpu::AddressingMode m_addrMode; //!< More info in the CPU documentation
    };

    /*!
     * \struct ConcatReg
     * \brief Concatenated registers represent a 16-bit value
     */
    struct ConcatReg {
        Cpu::Register msReg = Cpu::Register::A; //!< Most Significant Byte
        Cpu::Register lsReg = Cpu::Register::A; //!< Least Significant Byte
    };

    /*!
     * \enum ErrorType
     * \brief Lists data types (more info in the assembly language documentation)
     */
    enum class ErrorType { NONE, CIRC_DEPENDENCY, INVAL_EXPR, MISSING_TERM_CHAR, EXPECT_EXPR, INVAL_LABEL,
                           INVAL_CONCATREG_REG, INVAL_CONCATREG_SIZE, INVAL_DECVAL, INVAL_DECVAL_TOO_HIGH,
                           INVAL_HEXVAL, INVAL_HEXVAL_TOO_HIGH, UNAUTHOR_CHAR, EMPTY_STRING, INVAL_ADDR,
                           INVAL_EEPROM_ADDR_TOO_HIGH, INVAL_RAM_ADDR_TOO_HIGH, INVAL_DEF, DEF_ARG_NB, DEF_VAR_MISS,
                           DEF_ALREADY_EXIST, DEF_REPLAC_ERR, FILE_NOT_FOUND, NO_MAIN_FILE, INVAL_INC, INC_ARG_NB,
                           INC_FILE_NAME_MISS, FILE_NOT_READ, LABEL_ARG_NB, LABEL_INVAL_ARG, DATA_ARG_NB, DATA_VAR_MISS,
                           DATA_INVAL, INSTR_ARG_INVAL, INSTR_ARG_NB, MEM_SIZE, MEM_USE, INSTR_ALONE, LABEL_ALREADY_USED,
                           START_ROUTINE_MISSING, LABEL_START_REDEFINED, DATA_OVERWRITES_INSTR, MEM_USE_DEF_ROUTINES,
                           DATA_MEM_USE, SPLIT_MEM, LABEL_MEM_USE, DATA_OVERLAP, ROUTINE_OVERLAP, UNKNOWN_VARIABLE,
                           BIN_FILE_OPEN, MEM_SIZE_RAM, UNDEFINED_ADDRESS_EEPROM };

    const std::string errStr[] = { "No error", "Circular dependency on file ", "Invalid expression or string",
                                   "Missing '\"' termination character", "Expected expression",
                                   "Unexpected ':' character in label definition", "Invalid concatenated register expression",
                                   "Too many or too few registers concatenated", "Invalid decimal integer",
                                   "Decimal value > 255", "Invalid hexadecimal integer", "Hexadecimal value > 0xFF",
                                   "Use of unauthorized character ( ?,;:/!§*µù%^¨¤£&é~\"#'{(-|è`\\ç)=+} )",
                                   "Empty string", "Invalid hexadecimal address", "Hexadecimal address > 0xFFFFF",
                                   "Hexadecimal address > 0xFFFF", "Invalid define macro", "Too many or too few arguments for define macro",
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
                                   "Program size too large for the EEPROM (", "Exceeding memory size for instructions, try automatic address definition for variables",
                                   "Instructions must be part of routine block (defined after a label)", "This label name is already used",
                                   "\"_start\" routine is missing", "_start label cannot be redefined (address 0x300 by default)",
                                   "Data definition overlaps routine \"",
                                   "Defined routines exceed memory size", "Data definition exceeds memory size",
                                   "Error on free memory space split while analyzing defined data",
                                   "Routine exceeds memory size", "Data definition overlaps \"", "Routine overlaps \"",
                                   "Unknown variable", "Unable to write the binary file (",
                                   "Program size too large for the RAM, try targeting the EEPROM or unplugging it (",
                                   "Using variables with undefined addresses is prohibited with EEPROM" }; //!< More info in the assembly language documentation

    /*!
     * \class TokenItem
     * \brief Represents a token description
     *
     * Used by the assembler to store and process data from the code in the form of tokens.
     */
    class TokenItem
    {
        public:
            /*!
             * Constructs the token with a given string and determines its type.
             *
             * \param str String represented by the token
             * \sa setStr
             */
            TokenItem(std::string str);

            /*!
             * \brief Changes the string represented by the token and determines its type.
             *
             * \param str New string represented by the token
             * \sa TokenItem::TokenItem
             */
            void setStr(std::string str);

            /*!
             * \brief Sets the address associated with the token <i>(if it represents a label or a variable name)</i>.
             *
             * \param address 32-bit value (because it can be an EEPROM 20-bit address)
             */
            void setAsAddress(uint32_t address);

            /*!
             * \brief Returns the token type.
             */
            Token::TokenType getType();

            /*!
             * \brief Returns the instruction opcode associated with the token <i>(NOP if none)</i>.
             */
            Cpu::InstructionOpcode getInstructionOpcode();

            /*!
             * \brief Returns the register ID associated with the token <i>(A if none)</i>.
             */
            Cpu::Register getRegister();

            /*!
             * \brief Returns the concatenated register associated with the token <i>([AA] if none)</i>.
             */
            Token::ConcatReg getConcatRegs();

            /*!
             * \brief Returns the value associated with the token <i>(0x00 if none)</i>.
             */
            uint8_t getValue();

            /*!
             * \brief Returns the address associated with the token <i>(0x00000000 if none)</i>.
             */
            uint32_t getAddress();

            /*!
             * \brief Returns the label name associated with the token <i>("" if none)</i>.
             */
            std::string getLabelName();

            /*!
             * \brief Returns the variable associated with the token <i>("" if none)</i>.
             */
            std::string getVariableName();

            /*!
             * \brief Returns the string represented by the token.
             */
            std::string getStr();

            /*!
             * \brief Returns the error type stored in the token.
             */
            Token::ErrorType getErr();

            /*!
             * \brief Returns a string suited for console output.
             */
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
            Token::TokenType m_type;

            Cpu::InstructionOpcode m_instructionOpcode = Cpu::InstructionOpcode::NOP;
            Cpu::Register m_reg = Cpu::Register::A;
            Token::ConcatReg m_concatReg;
            uint8_t m_value = 0x00;
            uint32_t m_address = 0x0000;
            std::string m_labelName = "";

            std::string m_str;
            Token::ErrorType m_err;
    };

    /*!
     * \class TokenLine
     * \brief Represents the tokens in a given line
     *
     * Used by the assembler to store and process data for each line in the code.
     */
    class TokenLine
    {
        public:
            /*!
             * Constructs a line storing the token items that it's made of.
             */
            TokenLine();

            /*!
             * \brief Returns true if the line is valid (validity of arguments by keyword).
             */
            bool checkValidity(bool targetEeprom);

            /*!
             * \brief Returns the complete instruction struct (if any)
             *
             * \sa getDataType
             */
            Token::Instruction getInstruction();

            /*!
             * \brief Returns the data type (if any)
             *
             * \sa getInstruction
             */
            Token::DataType getDataType();

            /*!
             * \brief Returns true if the token line represents an instruction
             *
             * \sa isData, isLabel
             */
            bool isInstruction();

            /*!
             * \brief Returns true if the token line represents a data definition
             *
             * \sa isInstruction, isLabel
             */
            bool isData();

            /*!
             * \brief Returns true if the token line represents an label definition
             *
             * \sa isInstruction, isData
             */
            bool isLabel();

            std::vector<TokenItem> m_tokens;
            QString m_originFilePath;
            unsigned int m_originLineNb;
            std::string m_originStr;

            Token::Instruction m_instr;
            Token::DataType m_dataType;

            Token::ErrorType m_err;
            std::string m_additionalInfo; //!< Used to display more information after an error in the console output
    };

    /*!
     * \struct TokenFile
     * \brief Token representation of a code file (*.has)
     */
    struct TokenFile
    {
        std::vector<TokenLine> m_lines;

        QString m_fileName;
        QString m_filePath;
    };
}

#endif // TOKEN_H
