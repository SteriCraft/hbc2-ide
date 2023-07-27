
#ifndef ASSEMBLER_H
#define ASSEMBLER_H

#include <algorithm>

#include "console.h"
#include "projectManager.h"
#include "token.h"

#define NO_ADDRESS 0
#define PROGRAM_START_ADDRESS 0x0300
#define MEMORY_SIZE 0x10000 // 65'536 bytes
#define INSTRUCTION_SIZE 4

struct Error
{
    QString originFilePath;
    unsigned int originLineNb;
    ErrorType type;
    std::string additionalInfo;
};

struct Define
{
    std::string originalStr;
    std::string replacementStr;
    QString fileWhereDefined;
    unsigned int lineNbWhereDefined;
};

struct MemoryRange
{
    uint16_t begin;
    uint16_t end;
};

struct Variable
{
    std::string name;
    MemoryRange range;
    std::vector<Token> values;
    uint16_t size; // Size is not a function determined by range data, because some definition miss an address at first
    DataType type;

    QString originFile;
    unsigned originLineNb;
};

struct MemorySpace
{
    MemoryRange range;
    uint16_t size() { return range.end - range.begin + 1; }
};

struct RoutineBlock
{
    std::string labelName;
    std::vector<TokenLine> instructionLines;

    uint16_t size() { return (uint16_t)instructionLines.size() * INSTRUCTION_SIZE; }
    MemoryRange range;

    QString originFile;
    unsigned originLineNb;
};

class Assembler
{
    public:
        Assembler(Console *consoleOutput);

        bool isBinaryReady();
        QByteArray getBinaryData();
        bool assembleProject(Project *p);

    private:
        // Methods
        TokenFile retrieveContent(QString filePath);
        void logError();

        // Major pass 1 = TOKENS GENERATION
        void replaceTabsBySpaces(TokenFile &f);
        void removeComments(TokenFile &f);
        void removeEmptyLines(TokenFile &f);
        void fixIndentation(TokenFile &f);
        bool convertToTokens(TokenFile &f);

        // Major pass 2 = MACROS
        bool listDefines(TokenFile &f);
        bool processDefines();
        bool processIncludes();
        bool processIncludesInFile(TokenFile &f);

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
        TokenFile* findTokenFile(QString fileName);
        static bool variableAddressInferiorComparator(Variable a, Variable b);
        static bool routineBlockAddressInferiorComparator(RoutineBlock a, RoutineBlock b);
        static bool freeMemSpaceAddressInferiorComparator(MemorySpace a, MemorySpace b);
        bool splitFreeMemorySpace(unsigned int freeMemorySpaceIndex, unsigned int definedVariableIndex);
        bool doRangeOverlap(MemoryRange a, MemoryRange b);
        uint32_t getBinaryFromTokenLine(TokenLine* line);

        // Attributes
        QList<QString> m_filesPaths;
        std::vector<TokenFile> m_tokenFiles;
        TokenFile m_finalFile;

        std::vector<QString> m_includedFiles;
        std::vector<Define> m_defToProcess;
        std::vector<Variable> m_undefinedVars;
        std::vector<Variable> m_definedVars;
        std::vector<MemorySpace> m_freeMemorySpaces;
        std::vector<RoutineBlock> m_undefinedRoutineBlocks;
        std::vector<RoutineBlock> m_definedRoutineBlocks;

        //quint8 m_finalBinary[MEMORY_SIZE];
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

#endif // ASSEMBLER_H
