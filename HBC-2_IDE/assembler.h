#ifndef ASSEMBLER_H
#define ASSEMBLER_H

/*!
 * \file assembler.h
 * \brief The assembler that procudes binary for the HBC-2 based on its custom assembly language
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <algorithm>
#include "console.h"
#include "projectManager.h"
#include "token.h"

/*!
 * \namespace Assembly
 * \brief Assembler specifications.
 */
namespace Assembly
{
    constexpr int ADDRESS_NOT_SET = 0x0000; //!< Default value for later automatic address calculation

    /*!
     * \struct Error
     * \brief Needed information for Assembler to log an error.
     */
    struct Error
    {
        QString originFilePath;
        unsigned int originLineNb;
        Token::ErrorType type;
        std::string additionalInfo; //!< Used to display more information after an error in the console output
    };

    /*!
     * \struct Define
     * \brief Stores informations describing a define.
     */
    struct Define
    {
        std::string originalStr;
        std::string replacementStr;
        QString fileWhereDefined;
        unsigned int lineNbWhereDefined;
    };

    /*!
     * \struct MemoryRange
     * \brief First and last address (inclusive).
     */
    struct MemoryRange
    {
        uint16_t begin; //!< Inclusive
        uint16_t end; //!< Inclusive
    };

    /*!
     * \struct Variable
     * \brief Stores informations describing a variable.
     */
    struct Variable
    {
        std::string name;
        MemoryRange range;
        std::vector<Token::TokenItem> values; //!< Determined and checked before creation of the variable
        uint16_t size; //!< Unlike MemorySpace and others, size may be undefined (for automatic address calculation), so it's not a function
        Token::DataType type;

        QString originFile;
        unsigned originLineNb;
    };

    /*!
     * \struct MemorySpace
     * \brief Range and size.
     */
    struct MemorySpace
    {
        MemoryRange range;
        uint16_t size() { return range.end - range.begin + 1; }
    };

    /*!
     * \struct RoutineBlock
     * \brief Stores informations describing a routine (name, instructions, size and range).
     */
    struct RoutineBlock
    {
        std::string labelName;
        std::vector<Token::TokenLine> instructionLines;

        uint16_t size() { return (uint16_t)instructionLines.size() * Cpu::INSTRUCTION_SIZE; }
        MemoryRange range;

        QString originFile;
        unsigned originLineNb;
    };

    /*!
     * \class Assembler
     * \brief Singleton of the assembler
     *
     * Produces a binary output for a project, usable in the emulator or a real machine.
     *
     * Pressing F5 will assemble the active project.
     */
    class Assembler
    {
        static Assembler *m_singleton;

        public:
            /*!
             * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
             *
             * \param consoleOutput Pointer to Console
             */
            static Assembler* getInstance(Console *consoleOutput);

            /*!
             * \return <b>true</b> if the assembly succeeded and binary data is available
             */
            bool isBinaryReady();

            /*!
             * \return binary data after assembly
             */
            QByteArray getBinaryData();

            /*!
             * \brief Assembles the project and returns true it succeeded.
             *
             * Prompts in the console output statistics and error (if any).
             *
             * \param p Pointer to the project to assemble
             */
            bool assembleProject(std::shared_ptr<Project> p);

        private:
            Assembler(Console *consoleOutput);

            // Methods
            Token::TokenFile retrieveContent(QString filePath);
            void logError();

            // Major pass 1 = TOKENS GENERATION
            void replaceTabsBySpaces(Token::TokenFile &f);
            void removeComments(Token::TokenFile &f);
            void removeEmptyLines(Token::TokenFile &f);
            void fixIndentation(Token::TokenFile &f);
            bool convertToTokens(Token::TokenFile &f);

            // Major pass 2 = MACROS
            bool listDefines(Token::TokenFile &f);
            bool processDefines();
            bool processIncludes();
            bool processIncludesInFile(Token::TokenFile &f);

            // Major pass 3 = ARGUMENT VALIDITY
            bool checkArgumentsValidity();

            // Major pass 4 = DATA ANALYSIS
            bool listVariables();
            bool calculateMemoryUsage();

            // Major pass 5 = LABEL ANALYSIS
            bool constructRoutineBlocks();
            bool findFreeMemorySpaces();
            bool calculateRoutineBlocksAddresses();

            // Major pass 6 = DATA PROCESS
            bool calculateVariablesAddresses();
            bool replaceVariablesByAddresses();

            // Major pass 7 = BINARY CONVERSION
            bool convertTokensToBinary();
            bool convertDataToBinary();
            bool saveBinaryToFile();

            // Utils
            Token::TokenFile* findTokenFile(QString fileName);
            static bool variableAddressInferiorComparator(Variable a, Variable b);
            static bool routineBlockAddressInferiorComparator(RoutineBlock a, RoutineBlock b);
            static bool freeMemSpaceAddressInferiorComparator(MemorySpace a, MemorySpace b);
            bool splitFreeMemorySpace(unsigned int freeMemorySpaceIndex, unsigned int definedVariableIndex);
            bool doRangeOverlap(MemoryRange a, MemoryRange b);
            uint32_t getBinaryFromTokenLine(Token::TokenLine* line);

            // Attributes
            QList<QString> m_filesPaths;
            std::vector<Token::TokenFile> m_tokenFiles;
            Token::TokenFile m_finalFile;

            std::vector<QString> m_includedFiles;
            std::vector<Define> m_defToProcess;
            std::vector<Variable> m_undefinedVars;
            std::vector<Variable> m_definedVars;
            std::vector<MemorySpace> m_freeMemorySpaces;
            std::vector<RoutineBlock> m_undefinedRoutineBlocks;
            std::vector<RoutineBlock> m_definedRoutineBlocks;

            QByteArray m_finalBinary;
            bool m_binaryReady;

            unsigned int m_definesCount;
            unsigned int m_totalTokenCount;
            unsigned int m_variablesCount;
            unsigned int m_dataMemoryUse;
            unsigned int m_totalMemoryUse;
            unsigned int m_routineBlocksCount;

            Error m_error;

            Console *m_consoleOutput;
    };
}

#endif // ASSEMBLER_H
