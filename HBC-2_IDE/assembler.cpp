#include "assembler.h"
#include "eeprom.h"

using namespace Assembly;

Assembler* Assembler::m_singleton = nullptr;

Assembler* Assembler::getInstance(Console *consoleOutput)
{
    if (m_singleton == nullptr)
        m_singleton = new Assembler(consoleOutput);

    return m_singleton;
}

Assembler::Assembler(Console *consoleOutput)
{
    m_consoleOutput = consoleOutput;
    m_binaryReady = false;
}

bool Assembler::isBinaryReady()
{
    return m_binaryReady;
}

QByteArray Assembler::getBinaryData()
{
    return m_finalBinary.binaryData;
}

BinaryWithSymbols Assembler::getBinaryDataWithSymbols()
{
    return m_finalBinary;
}

ByteDebugSymbol Assembler::getSymbolFromAddress(Word address)
{
    return m_finalBinary.origin[address];
}

QString Assembler::getBinaryFilePath()
{
    return m_finalBinary.binaryFilePath;
}

bool Assembler::isAssembledForEeprom()
{
    return m_finalBinary.binaryData.size() == Eeprom::MEMORY_SIZE;
}

std::vector<Word> Assembler::getBreakpointsAddresses(std::vector<std::pair<QString, std::vector<int>>> filesBreakpoints)
{
    std::vector<Word> breakpoints;

    QString fileWithBreakpointsPath;
    int comparedLine;
    for (unsigned int i(0); i < filesBreakpoints.size(); i++)
    {
        fileWithBreakpointsPath = filesBreakpoints[i].first;

        for (unsigned int j(0); j < filesBreakpoints[i].second.size(); j++)
        {
            comparedLine = filesBreakpoints[i].second[j];

            for (unsigned int t(Cpu::PROGRAM_START_ADDRESS); t < m_finalBinary.origin.size(); t += 4) // Only the first byte has an associated origin
            {
                if (m_finalBinary.origin[t].filePath == fileWithBreakpointsPath && m_finalBinary.origin[t].lineNb == comparedLine)
                {
                    breakpoints.push_back(t);
                    break;
                }
            }
        }
    }

    return breakpoints;
}

bool Assembler::assembleProject(std::shared_ptr<Project> p, bool targetEeprom)
{
    m_consoleOutput->clear();
    m_consoleOutput->log("Assembling project \"" + p->getName() + "\"...");
    m_consoleOutput->returnLine();


    // Clear attributes because previous assemblies may have happened
    m_definesCount = 0;
    m_totalTokenCount = 0;
    m_variablesCount = 0;
    m_dataMemoryUse = 0;
    m_totalMemoryUse = 0;
    m_routineBlocksCount = 0;

    m_includedFiles.clear();
    m_defToProcess.clear();
    m_definedVars.clear();
    m_undefinedVars.clear();
    m_freeMemorySpaces.clear();
    m_routineBlocks.clear();

    m_error.originFilePath = "";
    m_error.originLineNb = 0;
    m_error.type = Token::ErrorType::NONE;
    m_error.additionalInfo = "";

    m_finalFile.m_lines.clear();
    m_tokenFiles.clear();

    initBinary(targetEeprom);


// === INIT ===
    // List all files in the project
    m_filesPaths = p->getFilesPaths();

    // Retrieve their content
    for (int i(0); i < m_filesPaths.count(); i++)
    {
        m_tokenFiles.push_back(retrieveContent(m_filesPaths[i]));

        if (m_error.type == Token::ErrorType::FILE_NOT_READ)
        {
            m_consoleOutput->log("Assembly failed");
            m_consoleOutput->returnLine();
            return false;
        }
    }


// === MAJOR PASS 1: Converts all files contents as tokens ===
    m_consoleOutput->log("Converting source code in tokens...");

    m_totalTokenCount = 0;

    for (unsigned int i(0); i < m_tokenFiles.size(); i++)
    {
// -> Minor pass 1: Replace tabs by 4 spaces
        replaceTabsBySpaces(m_tokenFiles[i]);

// -> Minor pass 2: Removes comments
        removeComments(m_tokenFiles[i]);

// -> Minor pass 3: Removes empty lines
        removeEmptyLines(m_tokenFiles[i]);

// -> Minor pass 4: Removes uneccesary tabs and spaces
        fixIndentation(m_tokenFiles[i]);

// -> Minor pass 5: Converts the result as general tokens
        if (!convertToTokens(m_tokenFiles[i]))
        {
            m_consoleOutput->log("Assembly failed");
            m_consoleOutput->returnLine();
            return false;
        }
    }

    m_consoleOutput->log(std::to_string(m_totalTokenCount) + " tokens identified");
    m_consoleOutput->returnLine();


// === MAJOR PASS 2: Process all preprocessor directives (.include and .define) ===
    m_consoleOutput->log("Processing define macros...");

// -> Minor pass 1: List all ".define" macros
    m_defToProcess.clear();
    for (unsigned int i(0); i < m_tokenFiles.size(); i++)
    {
        if (!listDefines(m_tokenFiles[i]))
        {
            m_consoleOutput->log("Assembly failed");
            m_consoleOutput->returnLine();
            return false;
        }
    }

// -> Minor pass 2: Process all ".define" macros
    if (!processDefines())
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

    m_consoleOutput->log(std::to_string(m_definesCount) + " define macro" + ((m_definesCount > 1) ? "s" : "") + " found and executed");
    m_consoleOutput->returnLine();

// -> Minor pass 3: Process all ".include" macros, starting in file "main.has"
    m_consoleOutput->log("Processing include macros...");

    if (!processIncludes())
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

    m_consoleOutput->log(std::to_string(m_includedFiles.size()) + " file" + ((m_includedFiles.size() > 1) ? "s" : "") + " included");
    m_consoleOutput->returnLine();

    m_finalFile.m_fileName = p->getName();
    m_finalFile.m_filePath = p->getPath();


// /!\ Working with one giant token file from now on /!\


// === MAJOR PASS 3: Argument validity ===
    m_consoleOutput->log("Checking tokens validity...");

    if (!checkArgumentsValidity(targetEeprom))
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

    m_consoleOutput->log("All tokens are valid");
    m_consoleOutput->returnLine();


// === MAJOR PASS 4: Data analysis ===
    m_consoleOutput->log("Listing variables...");

// -> Minor pass 1: List defined and undefined data definitions
    if (!listVariables(targetEeprom))
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

    m_consoleOutput->log(std::to_string(m_variablesCount) + " variables identified");
    m_consoleOutput->log("Total data memory usage: " + std::to_string(m_dataMemoryUse) + " bytes");
    m_consoleOutput->returnLine();

// -> Minor pass 2: Calculate total memory usage
    if (!calculateMemoryUsage(targetEeprom))
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }


// === MAJOR PASS 5: Label analysis ===
    m_consoleOutput->log("Identifying routine blocks...");

// -> Minor pass 1: Construct routine blocks, calculate their sizes and list labels
    if (!constructRoutineBlocks(targetEeprom))
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

// -> Minor pass 2: Calculate routines addresses (in a row)
    if (!calculateRoutineBlocksAddresses())
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

    m_consoleOutput->log(std::to_string(m_routineBlocksCount) + " routine block" + ((m_routineBlocksCount > 1) ? "s" : "") + " detected.");
    m_consoleOutput->returnLine();


// === MAJOR PASS 6: Data process ===
    m_consoleOutput->log("Check variable name uses validity...");

// -> Minor pass 1: List free memory spaces between routines and defined .data
    if (!findFreeMemorySpaces(targetEeprom))
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

// -> Minor pass 2: Calculate undefined data addresses (find first big enough free memory space)
    if (!calculateVariablesAddresses())
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

// -> Minor pass 2: Replace all vars in tokens with associated addresses
    if (!replaceVariablesByAddresses())
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

    m_consoleOutput->log("All uses of variable names are valid");
    m_consoleOutput->returnLine();


// === MAJOR PASS 7: Conversion of tokens to binary ===
    m_consoleOutput->log("Writing binary file...");

// -> Minor pass 1: Routine blocks conversion
    if (!convertTokensToBinary())
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

    // UNDER CONSTRUCTION
// -> Minor pass 2: Data integration
    if (!convertDataToBinary())
    {
        m_consoleOutput->log("Assembly failed");
        m_consoleOutput->returnLine();
        return false;
    }

// -> Minor pass 3: Binary save in file (only if the EEPROM is the target)
    if (targetEeprom)
    {
        if (!saveBinaryToFile())
        {
            m_consoleOutput->log("Assembly failed");
            m_consoleOutput->returnLine();
            return false;
        }

        m_consoleOutput->log("EEPROM binary file written");
    }

    m_consoleOutput->log("Assembly terminated successfuly");
    m_consoleOutput->log("Memory usage: " + std::to_string(m_totalMemoryUse) + " / " + std::to_string(targetEeprom ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE) + " bytes (" + std::to_string(m_totalMemoryUse * 100 / (targetEeprom ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE)) + " %)");
    m_consoleOutput->returnLine();

    m_binaryReady = true;
    p->setAssembled(true);

    return true;
}

Token::TokenFile Assembler::retrieveContent(QString filePath)
{
    Token::TokenFile tFile;
    QFile file(filePath);
    unsigned int lineNb(1);
    Token::TokenLine tLine;

    if (file.open(QIODevice::ReadOnly)) // Reading file line by line
    {
        QTextStream in(&file);

        // File info
        tFile.m_fileName = QFileInfo(filePath).fileName();
        tFile.m_filePath = filePath;

        // File content
        while (!in.atEnd())
        {
            tLine.m_originFilePath = filePath;
            tLine.m_originLineNb = lineNb;
            tLine.m_originStr = in.readLine().toStdString();

            tFile.m_lines.push_back(tLine);

            lineNb++;
        }
    }
    else
    {
        m_error.originFilePath = "";
        m_error.originLineNb = 0;
        m_error.type = Token::ErrorType::FILE_NOT_READ;
        m_error.additionalInfo = "";
    }

    return tFile;
}

void Assembler::logError()
{
    if (!m_error.originFilePath.isEmpty())
        m_consoleOutput->log("Error line " + std::to_string(m_error.originLineNb) + " of file "
                             + QFileInfo(m_error.originFilePath).fileName().toStdString());

    m_consoleOutput->log(Token::errStr[(int)m_error.type] + m_error.additionalInfo);
    m_consoleOutput->returnLine();
}

void Assembler::initBinary(bool targetEeprom)
{
    int memorySize(0);

    m_finalBinary.binaryData.clear();
    m_finalBinary.origin.clear();
    m_finalBinary.binaryFilePath = ""; // Will be set by saveBinaryToFile() if the EEPROM is the memory target

    memorySize = (targetEeprom ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE);

    quint8 nullChar(0);
    for (unsigned int i(0); i < memorySize; i++)
    {
        ByteDebugSymbol newSymbol;

        m_finalBinary.binaryData.push_back(nullChar);
        m_finalBinary.origin.push_back(newSymbol);
    }
}

// -- Major pass 1 --
void Assembler::replaceTabsBySpaces(Token::TokenFile &f)
{
    size_t tabPos;

    for (unsigned int i(0); i < f.m_lines.size(); i++)
    {
        tabPos = f.m_lines[i].m_originStr.find('\t');

        while (tabPos != std::string::npos)
        {
            f.m_lines[i].m_originStr = f.m_lines[i].m_originStr.replace(tabPos, 1, "    ");

            tabPos = f.m_lines[i].m_originStr.find('\t');
        }
    }
}

void Assembler::removeComments(Token::TokenFile &f)
{
    std::string *line; // Using a pointer to directly modify the token line string
    size_t commaPos;

    for (unsigned int i(0); i < f.m_lines.size(); i++)
    {
        line = &(f.m_lines[i].m_originStr);
        commaPos = line->find(';');

        if (commaPos != std::string::npos) // Comment found
            *line = line->substr(0, commaPos);
    }
}

void Assembler::removeEmptyLines(Token::TokenFile &f)
{
    bool empty;
    std::string *line; // Using a pointer to directly modify the token line string

    for (unsigned int i(0); i < f.m_lines.size(); i++)
    {
        empty = true;
        line = &(f.m_lines[i].m_originStr);

        // Check if the line is empty or just full of spaces (which is considered empty too)
        if (!line->empty())
        {
            for (unsigned int j(0); j < line->size(); j++)
            {
                if (line->at(j) != ' ')
                {
                    empty = false;
                    break;
                }
            }
        }

        // Actually erase the line
        if (empty)
        {
            f.m_lines.erase(f.m_lines.begin() + i);
            i--;
        }
    }
}

void Assembler::fixIndentation(Token::TokenFile &f)
{
    std::string *line; // Using a pointer to directly modify the token line string
    bool inbetweenQuotation(false);

    for (unsigned int i(0); i < f.m_lines.size(); i++)
    {
        line = &(f.m_lines[i].m_originStr);

        // Remove spaces at line's start
        while (line->at(0) == ' ')
            *line = line->substr(1);

        // Remove spaces and tabs at line's end
        while (line->back() == ' ')
            *line = line->substr(0, line->size() - 1);

        // Remove spaces and tabs before and after commas, unless in between " characters
        for (size_t t(0); t < line->size(); t++)
        {
            if (line->at(t) == '\"')
                inbetweenQuotation = !inbetweenQuotation;
            else if (line->at(t) == ',' && !inbetweenQuotation)
            {
                while (t - 1 > 0) // Erase spaces before comma
                {
                    if (line->at(t - 1) != ' ')
                        break;

                    line->erase(t - 1, 1);
                    t--;
                }

                while (t + 1 < line->size()) // Erase spaces after comma
                {
                    if (line->at(t + 1) != ' ')
                        break;

                    line->erase(t + 1, 1);
                }
            }
            else if (line->at(t) == ' ') // Remove multiple spaces in a row
            {
                while (t + 1 < line->size())
                {
                    if (line->at(t + 1) != ' ')
                        break;

                    line->erase(t + 1, 1);
                }
            }
        }
    }
}

bool Assembler::convertToTokens(Token::TokenFile &f)
{
    Token::TokenLine *tLine;
    Token::TokenItem newToken("");
    std::string line;
    std::string tokenStr;
    bool firstLabelFound(false);

    size_t nextComma, nextSpace, nextQuote;

    for (unsigned int i(0); i < f.m_lines.size(); i++)
    {
        tLine = &(f.m_lines[i]);
        line = tLine->m_originStr;

        for (size_t t(0); t < line.size(); t++)
        {
            nextComma = line.find(',',  t);
            nextSpace = line.find(' ',  t);
            nextQuote = line.find('\"', t);

            if (nextComma == (line.size() - 1))
            {
                m_error.originFilePath = tLine->m_originFilePath;
                m_error.originLineNb = tLine->m_originLineNb;
                m_error.type = Token::ErrorType::EXPECT_EXPR;
                m_error.additionalInfo = "";

                logError();
                return false;
            }

            if (nextComma == std::string::npos && nextSpace == std::string::npos && nextQuote == std::string::npos) // No more bounding char until EOL
            {
                tokenStr = line.substr(t);

                if (tokenStr.empty())
                {
                    m_error.originFilePath = tLine->m_originFilePath;
                    m_error.originLineNb = tLine->m_originLineNb;
                    m_error.type = Token::ErrorType::EXPECT_EXPR;
                    m_error.additionalInfo = "";

                    logError();
                    return false;
                }

                newToken = Token::TokenItem(tokenStr);

                t = line.size(); //break;
            }
            else if ((nextComma < nextSpace) && (nextComma < nextQuote)) // Closest bounding char is ','
            {
                tokenStr = line.substr(t, nextComma - t);

                if (tokenStr.empty())
                {
                    m_error.originFilePath = tLine->m_originFilePath;
                    m_error.originLineNb = tLine->m_originLineNb;
                    m_error.type = Token::ErrorType::EXPECT_EXPR;
                    m_error.additionalInfo = "";

                    logError();
                    return false;
                }

                newToken = Token::TokenItem(tokenStr);

                t = nextComma;
            }
            else if ((nextSpace < nextComma) && (nextSpace < nextQuote)) // Closest bounding char is ' '
            {
                tokenStr = line.substr(t, nextSpace - t);

                if (tokenStr.empty())
                {
                    m_error.originFilePath = tLine->m_originFilePath;
                    m_error.originLineNb = tLine->m_originLineNb;
                    m_error.type = Token::ErrorType::EXPECT_EXPR;
                    m_error.additionalInfo = "";

                    logError();
                    return false;
                }

                newToken = Token::TokenItem(tokenStr);

                t = nextSpace;
            }
            else/* if ((nextQuote < nextComma) && (nextQuote < nextSpace))*/ // Closest bounding char is '"'
            {
                if (nextQuote != t)
                {
                    m_error.originFilePath = tLine->m_originFilePath;
                    m_error.originLineNb = tLine->m_originLineNb;
                    m_error.type = Token::ErrorType::INVAL_EXPR;
                    m_error.additionalInfo = "";

                    logError();
                    return false;
                }

                nextQuote = line.find('\"', t + 1); // Looking for the ending quote
                if (nextQuote == std::string::npos)
                {
                    m_error.originFilePath = tLine->m_originFilePath;
                    m_error.originLineNb = tLine->m_originLineNb;
                    m_error.type = Token::ErrorType::MISSING_TERM_CHAR;
                    m_error.additionalInfo = "";

                    logError();
                    return false;
                }

                tokenStr = line.substr(t, nextQuote - t + 1);
                newToken = Token::TokenItem(tokenStr);

                t = nextQuote + 1;
            }

            if (newToken.getErr() != Token::ErrorType::NONE)
            {
                m_error.originFilePath = tLine->m_originFilePath;
                m_error.originLineNb = tLine->m_originLineNb;
                m_error.type = newToken.getErr();
                m_error.additionalInfo = "";

                logError();
                return false;
            }

            // Checking if the first instruction of the token file isn't alone
            if (newToken.getType() == Token::TokenType::LABEL)
            {
                firstLabelFound = true;
            }
            else if (newToken.getType() == Token::TokenType::INSTR && !firstLabelFound)
            {
                m_error.originFilePath = tLine->m_originFilePath;
                m_error.originLineNb = tLine->m_originLineNb;
                m_error.type = Token::ErrorType::INSTR_ALONE;
                m_error.additionalInfo = "";

                logError();
                return false;
            }

            tLine->m_tokens.push_back(newToken);
            m_totalTokenCount++;
        }
    }

    return true;
}

// -- Major pass 2 --
bool Assembler::listDefines(Token::TokenFile &f)
{
    bool toProcess;
    Define def;

    // Validity pass
    for (unsigned int i(0); i < f.m_lines.size(); i++)
    {
        toProcess = f.m_lines[i].m_tokens[0].getType() == Token::TokenType::DEFINE;

        // Use of define elsewhere than on first token
        for (unsigned int j(1); j < f.m_lines[i].m_tokens.size(); j++)
        {
            if (f.m_lines[i].m_tokens[j].getType() == Token::TokenType::DEFINE)
            {
                m_error.originFilePath = f.m_lines[i].m_originFilePath;
                m_error.originLineNb = f.m_lines[i].m_originLineNb;
                m_error.type = Token::ErrorType::INVAL_DEF;
                m_error.additionalInfo = "";

                logError();
                return false;
            }
        }

        if (toProcess)
        {
            // Invalid number of arguments
            if (f.m_lines[i].m_tokens.size() != 3)
            {
                m_error.originFilePath = f.m_lines[i].m_originFilePath;
                m_error.originLineNb = f.m_lines[i].m_originLineNb;
                m_error.type = Token::ErrorType::DEF_ARG_NB;
                m_error.additionalInfo = "";

                logError();
                return false;
            }

            // Variable to replace not set
            if (f.m_lines[i].m_tokens[1].getType() != Token::TokenType::VAR)
            {
                m_error.originFilePath = f.m_lines[i].m_originFilePath;
                m_error.originLineNb = f.m_lines[i].m_originLineNb;
                m_error.type = Token::ErrorType::DEF_VAR_MISS;
                m_error.additionalInfo = "";

                logError();
                return false;
            }

            // Second argument can be anything

            // Storage in defToProcess list
            def.originalStr = f.m_lines[i].m_tokens[1].getVariableName();
            def.replacementStr = f.m_lines[i].m_tokens[2].getStr();
            def.fileWhereDefined = f.m_lines[i].m_originFilePath;
            def.lineNbWhereDefined = f.m_lines[i].m_originLineNb;

            // Check if already defined elsewhere
            for (unsigned int j(0); j < m_defToProcess.size(); j++)
            {
                if (m_defToProcess[j].originalStr == def.originalStr)
                {
                    m_error.originFilePath = f.m_lines[i].m_originFilePath;
                    m_error.originLineNb = f.m_lines[i].m_originLineNb;
                    m_error.type = Token::ErrorType::DEF_ALREADY_EXIST;
                    m_error.additionalInfo = "\"" + QFileInfo(m_defToProcess[j].fileWhereDefined).fileName().toStdString()
                                             + "\"" + " at line " + std::to_string(m_defToProcess[j].lineNbWhereDefined);

                    logError();
                    return false;
                }
            }

            m_defToProcess.push_back(def);
            f.m_lines.erase(f.m_lines.begin() + i);
            i--;
        }
    }

    m_definesCount = (unsigned int)m_defToProcess.size();

    return true;
}

bool Assembler::processDefines()
{
    Token::TokenItem *toCheckTk;

    for (unsigned int f(0); f < m_tokenFiles.size(); f++)
    {
        for (unsigned int l(0); l < m_tokenFiles[f].m_lines.size(); l++)
        {
            for (unsigned int t(0); t < m_tokenFiles[f].m_lines[l].m_tokens.size(); t++)
            {
                toCheckTk = &(m_tokenFiles[f].m_lines[l].m_tokens[t]);

                for (unsigned int i(0); i < m_defToProcess.size(); i++)
                {
                    if (toCheckTk->getStr() == m_defToProcess[i].originalStr)
                    {
                        toCheckTk->setStr(m_defToProcess[i].replacementStr);

                        if (toCheckTk->getErr() != Token::ErrorType::NONE)
                        {
                            m_error.originFilePath = m_tokenFiles[f].m_lines[l].m_originFilePath;
                            m_error.originLineNb = m_tokenFiles[f].m_lines[l].m_originLineNb;
                            m_error.type = Token::ErrorType::DEF_REPLAC_ERR;
                            m_error.additionalInfo = "\"" + QFileInfo(m_defToProcess[i].fileWhereDefined).fileName().toStdString()
                                                     + "\"" + " at line " + std::to_string(m_defToProcess[i].lineNbWhereDefined);

                            logError();
                            return false;
                        }
                    }
                }
            }
        }
    }

    return true;
}

bool Assembler::processIncludes()
{
    // Looking for "main.has" file
    Token::TokenFile *mainTokenFile = findTokenFile("main.has");

    if (mainTokenFile == nullptr)
    {
        m_error.originFilePath = "";
        m_error.originLineNb = 0;
        m_error.type = Token::ErrorType::NO_MAIN_FILE;
        logError();

        return false;
    }

    // Preparing the final token list
    m_finalFile.m_lines.clear();
    m_includedFiles.clear();

    // Parse recursively
    if (!processIncludesInFile(*mainTokenFile))
    {
        logError();
        return false;
    }

    return true;
}

bool Assembler::processIncludesInFile(Token::TokenFile &f)
{
    bool toProcess;
    QString fileToInclude;
    Token::TokenFile *tokenFileToInclude;

    // Parse the file
    for (unsigned int i(0); i < f.m_lines.size(); i++)
    {
        fileToInclude = "";
        tokenFileToInclude = nullptr;

        toProcess = f.m_lines[i].m_tokens[0].getType() == Token::TokenType::INCLUDE;

        // Use of include elsewhere than on first token
        for (unsigned int j(1); j < f.m_lines[i].m_tokens.size(); j++)
        {
            if (f.m_lines[i].m_tokens[j].getType() == Token::TokenType::INCLUDE)
            {
                m_error.originFilePath = f.m_lines[i].m_originFilePath;
                m_error.originLineNb = f.m_lines[i].m_originLineNb;
                m_error.type = Token::ErrorType::INVAL_INC;
                m_error.additionalInfo = "";
                return false;
            }
        }

        if (toProcess)
        {
            // -- Check for argument validity --
            // Invalid number of arguments
            if (f.m_lines[i].m_tokens.size() != 2)
            {
                m_error.originFilePath = f.m_lines[i].m_originFilePath;
                m_error.originLineNb = f.m_lines[i].m_originLineNb;
                m_error.type = Token::ErrorType::INC_ARG_NB;
                m_error.additionalInfo = "";
                return false;
            }

            // String indicating the name of the file to include
            if (f.m_lines[i].m_tokens[1].getType() != Token::TokenType::STRING)
            {
                m_error.originFilePath = f.m_lines[i].m_originFilePath;
                m_error.originLineNb = f.m_lines[i].m_originLineNb;
                m_error.type = Token::ErrorType::INC_FILE_NAME_MISS;
                m_error.additionalInfo = "";
                return false;
            }

            // Check if this file isn't already included
            fileToInclude = QString::fromStdString(f.m_lines[i].m_tokens[1].getStr());

            for (unsigned int j(0); j < m_includedFiles.size(); j++)
            {
                if (m_includedFiles[j] == fileToInclude)
                {
                    m_error.originFilePath = f.m_lines[i].m_originFilePath;
                    m_error.originLineNb = f.m_lines[i].m_originLineNb;
                    m_error.type = Token::ErrorType::CIRC_DEPENDENCY;
                    m_error.additionalInfo = "\"" + fileToInclude.toStdString() + "\"";
                    return false;
                }
            }

            // Finding the file to include
            tokenFileToInclude = findTokenFile(fileToInclude);

            if (tokenFileToInclude == nullptr)
            {
                m_error.originFilePath = f.m_lines[i].m_originFilePath;
                m_error.originLineNb = f.m_lines[i].m_originLineNb;
                m_error.type = Token::ErrorType::FILE_NOT_FOUND;
                m_error.additionalInfo = "\"" + fileToInclude.toStdString() + "\" can't be found";
                return false;
            }

            m_includedFiles.push_back(fileToInclude);

            // Parse recursively
            if (!processIncludesInFile(*tokenFileToInclude))
                return false;

            // No need to remove the line, because everything is copied in the final file but "include" macros
        }
        else
        {
            m_finalFile.m_lines.push_back(f.m_lines[i]);
        }
    }

    return true;
}

// -- Major pass 3 --
bool Assembler::checkArgumentsValidity(bool targetEeprom)
{
    for (unsigned int i(0); i < m_finalFile.m_lines.size(); i++)
    {
        if (!m_finalFile.m_lines[i].checkValidity(targetEeprom))
        {
            m_error.originFilePath = m_finalFile.m_lines[i].m_originFilePath;
            m_error.originLineNb = m_finalFile.m_lines[i].m_originLineNb;
            m_error.type = m_finalFile.m_lines[i].m_err;
            m_error.additionalInfo = m_finalFile.m_lines[i].m_additionalInfo;

            logError();
            return false;
        }
    }

    return true;
}

// -- Major pass 4 --
bool Assembler::listVariables(bool targetEeprom)
{
    Token::TokenLine *line;
    Variable var;

    for (unsigned int i(0); i < m_finalFile.m_lines.size(); i++)
    {
        line = &(m_finalFile.m_lines[i]);
        var.values.clear();
        var.size = 0;

        if (line->isData())
        {
            var.name = line->m_tokens[1].getVariableName();
            var.type = line->getDataType();

            if (line->getDataType() == Token::DataType::SINGLE_VALUE_DEFINED)
            {
                var.values.push_back(line->m_tokens[3]);

                var.size = 1;
            }
            else if (line->getDataType() == Token::DataType::SINGLE_VALUE_UNDEFINED)
            {
                var.values.push_back(line->m_tokens[2]);

                var.size = 1;
            }
            else if (line->getDataType() == Token::DataType::MULTIPLE_VALUES_DEFINED)
            {
                for (unsigned int j(3); j < line->m_tokens.size(); j++)
                {
                    var.values.push_back(line->m_tokens[j]);
                }

                var.size = (uint32_t)line->m_tokens.size() - 3;
            }
            else if (line->getDataType() == Token::DataType::MULTIPLE_VALUES_UNDEFINED)
            {
                for (unsigned int j(2); j < line->m_tokens.size(); j++)
                {
                    var.values.push_back(line->m_tokens[j]);
                }

                var.size = (uint32_t)line->m_tokens.size() - 2;
            }
            else if (line->getDataType() == Token::DataType::STRING_DEFINED)
            {
                for (unsigned int j(3); j < line->m_tokens.size(); j++)
                {
                    var.values.push_back(line->m_tokens[j]);
                }

                var.size = (uint32_t)var.values[0].getStr().size();
            }
            else // STRING_UNDEFINED
            {
                for (unsigned int j(2); j < line->m_tokens.size(); j++)
                {
                    var.values.push_back(line->m_tokens[j]);
                }

                var.size = (uint32_t)var.values[0].getStr().size();
            }

            m_dataMemoryUse += var.size;

            var.originFile = m_finalFile.m_lines[i].m_originFilePath;
            var.originLineNb = m_finalFile.m_lines[i].m_originLineNb;

            if (line->getDataType() == Token::DataType::STRING_DEFINED || line->getDataType() == Token::DataType::SINGLE_VALUE_DEFINED
             || line->getDataType() == Token::DataType::MULTIPLE_VALUES_DEFINED)
            {
                var.range.begin = line->m_tokens[2].getAddress();
                var.range.end = var.range.begin + var.size - 1;

                // Check if that new data definition do not overlap any other one
                for (unsigned int j(0); j < m_definedVars.size(); j++)
                {
                    if (doRangeOverlap(m_definedVars[j].range, var.range))
                    {
                        m_error.originFilePath = var.originFile;
                        m_error.originLineNb = var.originLineNb;
                        m_error.type = Token::ErrorType::DATA_OVERLAP;
                        m_error.additionalInfo = m_definedVars[j].name + "\"";

                        logError();
                        return false;
                    }
                }

                m_definedVars.push_back(var);
            }
            else
            {
                if (targetEeprom)
                {
                    m_error.originFilePath = var.originFile;
                    m_error.originLineNb = var.originLineNb;
                    m_error.type = Token::ErrorType::UNDEFINED_ADDRESS_EEPROM;
                    m_error.additionalInfo = "";

                    logError();
                    return false;
                }

                var.range.begin = Assembly::ADDRESS_NOT_SET;
                m_undefinedVars.push_back(var);
            }

            m_variablesCount++;

            m_finalFile.m_lines.erase(m_finalFile.m_lines.begin() + i);
            i--;
        }
    }

    // Insuring defined data definitions last address do not exceed memory size
    for (unsigned int i(0); i < m_definedVars.size(); i++)
    {
        if ((unsigned int)m_definedVars[i].range.begin + m_definedVars[i].size - 1 >= (targetEeprom ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE))
        {
            m_error.originFilePath = m_definedVars[i].originFile;
            m_error.originLineNb = m_definedVars[i].originLineNb;
            m_error.type = Token::ErrorType::DATA_MEM_USE;
            m_error.additionalInfo = "";

            logError();
            return false;
        }
    }

    return true;
}

bool Assembler::calculateMemoryUsage(bool targetEeprom)
{
    for (unsigned int i(0); i < m_finalFile.m_lines.size(); i++)
    {
        if (m_finalFile.m_lines[i].isInstruction())
            m_totalMemoryUse += 4;
    }

    for (unsigned int i(0); i < m_definedVars.size(); i++)
    {
        m_totalMemoryUse += m_definedVars[i].size;
    }

    for (unsigned int i(0); i < m_undefinedVars.size(); i++)
    {
        m_totalMemoryUse += m_undefinedVars[i].size;
    }

    if (targetEeprom)
    {
        if (m_totalMemoryUse >= Eeprom::MEMORY_SIZE)
        {
            m_error.originFilePath = "";
            m_error.originLineNb = 0;
            m_error.type = Token::ErrorType::MEM_SIZE;
            m_error.additionalInfo = " / " + std::to_string(Eeprom::MEMORY_SIZE) + " bytes";

            logError();
            return false;
        }
    }
    else
    {
        if (m_totalMemoryUse >= Ram::MEMORY_SIZE)
        {
            m_error.originFilePath = "";
            m_error.originLineNb = 0;
            m_error.type = Token::ErrorType::MEM_SIZE_RAM;
            m_error.additionalInfo = " / " + std::to_string(Ram::MEMORY_SIZE) + " bytes";

            logError();
            return false;
        }
    }

    return true;
}

// -- Major pass 5 --
bool Assembler::constructRoutineBlocks(bool targetEeprom)
{
    RoutineBlock block;
    Token::TokenLine *line(nullptr);
    std::string newLabel("");
    bool firstRun(true);

    for (unsigned int i(0); i < m_finalFile.m_lines.size(); i++)
    {
        line = &(m_finalFile.m_lines[i]);

        if (line->isInstruction())
        {
            if (firstRun)
            {
                m_error.originFilePath = line->m_originFilePath;
                m_error.originLineNb = line->m_originLineNb;
                m_error.type = Token::ErrorType::INSTR_ALONE;
                m_error.additionalInfo = "";

                logError();
                return false;
            }
            else
            {
                block.instructionLines.push_back(*line); // Where instructions are actually added to the routine block
            }
        }
        else if (line->isLabel())
        {
            if (!firstRun) // Second or more routine in the file
            {
                if (block.range.begin != ADDRESS_NOT_SET) // Defined address for the routine block
                {
                    if (((unsigned int)block.range.begin + block.size()) >= (targetEeprom ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE))
                    {
                        m_error.originFilePath = line->m_originFilePath;
                        m_error.originLineNb = line->m_originLineNb;
                        m_error.type = Token::ErrorType::LABEL_MEM_USE;
                        m_error.additionalInfo = "";

                        logError();
                        return false;
                    }

                    block.range.end = block.range.begin + block.size() - 1;
                }

                m_routineBlocks.push_back(block);
            }
            else // First routine block in the file
                firstRun = false;

            newLabel = line->m_tokens[0].getLabelName();

            // Check for same name in defined routine blocks
            for (unsigned int j(0); j < m_routineBlocks.size(); j++)
            {
                if (m_routineBlocks[j].labelName == newLabel)
                {
                    m_error.originFilePath = line->m_originFilePath;
                    m_error.originLineNb = line->m_originLineNb;
                    m_error.type = Token::ErrorType::LABEL_ALREADY_USED;
                    m_error.additionalInfo = "";

                    logError();
                    return false;
                }
            }

            // Reinit a new block
            block.labelName = newLabel;
            block.range.begin = (line->m_tokens.size() == 1) ? ADDRESS_NOT_SET : line->m_tokens[1].getAddress();
            block.instructionLines.clear();
            block.originFile = line->m_originFilePath;
            block.originLineNb = line->m_originLineNb;

            if (block.labelName == "_start")
            {
                block.range.begin = Cpu::PROGRAM_START_ADDRESS;
            }
        }
    }

    // Last routine storage
    if (block.size() > 0)
    {
        if (block.range.begin != ADDRESS_NOT_SET)
        {
            if (((unsigned int)block.range.begin + block.size()) >= (targetEeprom ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE))
            {
                m_error.originFilePath = line->m_originFilePath;
                m_error.originLineNb = line->m_originLineNb;
                m_error.type = Token::ErrorType::LABEL_MEM_USE;
                m_error.additionalInfo = "";

                logError();
                return false;
            }

            block.range.end = block.range.begin + block.size() - 1;
        }

        m_routineBlocks.push_back(block);
    }

    m_routineBlocksCount = (unsigned int)m_routineBlocks.size();

    // Find "_start" label
    bool startRoutineFound(false);
    for (unsigned int i(0); i < m_routineBlocks.size(); i++)
    {
        if (m_routineBlocks[i].labelName == "_start")
        {
            startRoutineFound = true;
            break;
        }
    }

    if (!startRoutineFound)
    {
        m_error.originFilePath = "";
        m_error.originLineNb = 0;
        m_error.type = Token::ErrorType::START_ROUTINE_MISSING;
        m_error.additionalInfo = "";

        logError();
        return false;
    }

    return true;
}

bool Assembler::calculateRoutineBlocksAddresses()
{
    uint32_t nextAvailableAddress(ADDRESS_NOT_SET);

    for (unsigned int i(0); i < m_routineBlocks.size(); i++)
    {
        if (m_routineBlocks[i].range.begin == ADDRESS_NOT_SET) // Undefined routine
        {
            m_routineBlocks[i].range.begin = nextAvailableAddress;
            m_routineBlocks[i].range.end = nextAvailableAddress + m_routineBlocks[i].size() - 1;

            if (m_routineBlocks[i].range.end >= Eeprom::MEMORY_SIZE)
            {
                m_error.originFilePath = "";
                m_error.originLineNb = 0;
                m_error.type = Token::ErrorType::MEM_USE;
                m_error.additionalInfo = "";

                logError();
                return false;
            }
        }

        nextAvailableAddress = m_routineBlocks[i].range.begin + m_routineBlocks[i].size();
    }

    // Check if any routine block overlaps another one
    for (unsigned int i(0); i < m_routineBlocks.size(); i++)
    {
        for (unsigned int j(0); j < m_routineBlocks.size(); j++)
        {
            if (i != j)
            {
                if (doRangeOverlap(m_routineBlocks[i].range, m_routineBlocks[j].range))
                {
                    m_error.originFilePath = m_routineBlocks[i].originFile;
                    m_error.originLineNb = m_routineBlocks[i].originLineNb;
                    m_error.type = Token::ErrorType::ROUTINE_OVERLAP;
                    m_error.additionalInfo = m_routineBlocks[j].labelName + "\"";

                    logError();
                    return false;
                }
            }
        }
    }

    // Check if any data definition overlaps a routine
    for (unsigned int i(0); i < m_routineBlocks.size(); i++)
    {
        for (unsigned int j(0); j < m_definedVars.size(); j++)
        {
            if (doRangeOverlap(m_routineBlocks[i].range, m_definedVars[j].range))
            {
                m_error.originFilePath = m_definedVars[j].originFile;
                m_error.originLineNb = m_definedVars[j].originLineNb;
                m_error.type = Token::ErrorType::DATA_OVERWRITES_INSTR;
                m_error.additionalInfo = m_routineBlocks[i].labelName + "\", try automatic data address calculation";

                logError();
                return false;
            }
        }
    }

    return true;
}

// -- Major pass 6 --
bool Assembler::findFreeMemorySpaces(bool targetEeprom)
{
    std::sort(m_definedVars.begin(), m_definedVars.end(), variableAddressInferiorComparator);
    std::sort(m_routineBlocks.begin(), m_routineBlocks.end(), routineAddressInferiorComparator);

    MemorySpace newFreeSpace;
    newFreeSpace.range.begin = Cpu::PROGRAM_START_ADDRESS;

    m_freeMemorySpaces.clear();

    for (unsigned int i(0); i < m_routineBlocks.size(); i++)
    {
        if (m_routineBlocks[i].range.begin > newFreeSpace.range.begin)
        {
            newFreeSpace.range.end = m_routineBlocks[i].range.begin - 1;
            m_freeMemorySpaces.push_back(newFreeSpace);
        }

        newFreeSpace.range.begin = m_routineBlocks[i].range.end + 1;
    }

    if (m_routineBlocks.back().range.end < (targetEeprom ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE))
    {
        newFreeSpace.range.end = (targetEeprom ? Eeprom::MEMORY_SIZE : Ram::MEMORY_SIZE) - 1;
        m_freeMemorySpaces.push_back(newFreeSpace);
    }

    // Update free memory spaces with defined data
    for (unsigned int i(0); i < m_definedVars.size(); i++)
    {
        // Update impacted free memory space
        for (unsigned int j(0); j < m_freeMemorySpaces.size(); j++)
        {
            if (doRangeOverlap(m_definedVars[i].range, m_freeMemorySpaces[j].range))
            {
                if (!splitFreeMemorySpace(j, i))
                {
                    m_error.originFilePath = "";
                    m_error.originLineNb = 0;
                    m_error.type = Token::ErrorType::SPLIT_MEM;
                    m_error.additionalInfo = "";

                    logError();
                    return false;
                }

                break;
            }
        }
    }

    return true;
}

bool Assembler::calculateVariablesAddresses()
{
    while (m_undefinedVars.size() > 0)
    {
        bool spaceFound(false);

        for (unsigned int j(0); j < m_freeMemorySpaces.size(); j++)
        {
            if (m_freeMemorySpaces[j].size() >= m_undefinedVars[0].size) // Large enough free memory space found
            {
                spaceFound = true;
                m_undefinedVars[0].range.begin = m_freeMemorySpaces[j].range.begin;
                m_undefinedVars[0].range.end = m_undefinedVars[0].range.begin + m_undefinedVars[0].size - 1;

                // Update free memory spaces
                if (m_undefinedVars[0].size == m_freeMemorySpaces[j].size())
                {
                    m_freeMemorySpaces.erase(m_freeMemorySpaces.begin() + j);
                }
                else
                {
                    m_freeMemorySpaces[j].range.begin += m_undefinedVars[0].size;
                }

                break;
            }
        }

        if (!spaceFound)
        {
            m_error.originFilePath = "";
            m_error.originLineNb = 0;
            m_error.type = Token::ErrorType::MEM_USE;
            m_error.additionalInfo = "";

            logError();
            return false;
        }

        m_definedVars.push_back(m_undefinedVars[0]);
        m_undefinedVars.erase(m_undefinedVars.begin());
    }

    std::sort(m_definedVars.begin(), m_definedVars.end(), variableAddressInferiorComparator);

    return true;
}

bool Assembler::replaceVariablesByAddresses()
{
    bool found(false);

    for (unsigned int i(0); i < m_routineBlocks.size(); i++)
    {
        for (unsigned int j(0); j < m_routineBlocks[i].instructionLines.size(); j++)
        {
            Token::TokenLine *line = &(m_routineBlocks[i].instructionLines[j]);

            for (unsigned int t(0); t < line->m_tokens.size(); t++)
            {
                if (line->m_tokens[t].getType() == Token::TokenType::VAR)
                {
                    // Search for equivalent variable
                    found = false;
                    for (unsigned int x(0); x < m_definedVars.size(); x++)
                    {
                        if (m_definedVars[x].name == line->m_tokens[t].getVariableName())
                        {
                            line->m_tokens[t].setAsAddress(m_definedVars[x].range.begin);

                            found = true;
                            break;
                        }
                    }

                    // Search for equivalent routine block
                    if (!found)
                    {
                        for (unsigned int x(0); x < m_routineBlocks.size(); x++)
                        {
                            if (m_routineBlocks[x].labelName == line->m_tokens[t].getVariableName())
                            {
                                line->m_tokens[t].setAsAddress(m_routineBlocks[x].range.begin);

                                found = true;
                                break;
                            }
                        }
                    }

                    if (!found)
                    {
                        m_error.originFilePath = line->m_originFilePath;
                        m_error.originLineNb = line->m_originLineNb;
                        m_error.type = Token::ErrorType::UNKNOWN_VARIABLE;
                        m_error.additionalInfo = "";

                        logError();
                        return false;
                    }
                }
                else if (line->m_tokens[t].getType() == Token::TokenType::ADDR_MSB)
                {
                    // Search for equivalent variable
                    found = false;
                    for (unsigned int x(0); x < m_definedVars.size(); x++)
                    {
                        if (m_definedVars[x].name == line->m_tokens[t].getLabelName())
                        {
                            line->m_tokens[t].setAsValue((uint8_t)(m_definedVars[x].range.begin >> 8));

                            found = true;
                            break;
                        }
                    }

                    // Search for equivalent routine block
                    if (!found)
                    {
                        for (unsigned int x(0); x < m_routineBlocks.size(); x++)
                        {
                            if (m_routineBlocks[x].labelName == line->m_tokens[t].getLabelName())
                            {
                                line->m_tokens[t].setAsValue((uint8_t)(m_routineBlocks[x].range.begin >> 8));

                                found = true;
                                break;
                            }
                        }
                    }

                    if (!found)
                    {
                        m_error.originFilePath = line->m_originFilePath;
                        m_error.originLineNb = line->m_originLineNb;
                        m_error.type = Token::ErrorType::UNKNOWN_VARIABLE;
                        m_error.additionalInfo = "";

                        logError();
                        return false;
                    }
                }
                else if (line->m_tokens[t].getType() == Token::TokenType::ADDR_LSB)
                {
                    std::string variableName(line->m_tokens[t].getStr().substr(0, line->m_tokens[t].getStr().size() - 4));

                    // Search for equivalent variable
                    found = false;
                    for (unsigned int x(0); x < m_definedVars.size(); x++)
                    {
                        if (m_definedVars[x].name == line->m_tokens[t].getLabelName())
                        {
                            line->m_tokens[t].setAsValue((uint8_t)m_definedVars[x].range.begin);

                            found = true;
                            break;
                        }
                    }

                    // Search for equivalent routine block
                    if (!found)
                    {
                        for (unsigned int x(0); x < m_routineBlocks.size(); x++)
                        {
                            if (m_routineBlocks[x].labelName == line->m_tokens[t].getLabelName())
                            {
                                line->m_tokens[t].setAsValue((uint8_t)m_routineBlocks[x].range.begin);

                                found = true;
                                break;
                            }
                        }
                    }

                    if (!found)
                    {
                        m_error.originFilePath = line->m_originFilePath;
                        m_error.originLineNb = line->m_originLineNb;
                        m_error.type = Token::ErrorType::UNKNOWN_VARIABLE;
                        m_error.additionalInfo = "";

                        logError();
                        return false;
                    }
                }
            }
        }
    }

    return true;
}

// -- Major pass 7 --
bool Assembler::convertTokensToBinary()
{
    Token::TokenLine *line(nullptr);
    uint32_t instructionAddress; // 32-bit because it could hold an EEPROM 20-bit address
    uint32_t instructionBinary;
    for (unsigned int i(0); i < m_routineBlocks.size(); i++)
    {
        instructionAddress = m_routineBlocks[i].range.begin;

        for (unsigned int j(0); j < m_routineBlocks[i].instructionLines.size(); j++)
        {
            line = &(m_routineBlocks[i].instructionLines[j]);

            instructionBinary = getBinaryFromTokenLine(line);

            m_finalBinary.binaryData[instructionAddress]     = ((instructionBinary & 0xFF000000) >> 24);
            m_finalBinary.binaryData[instructionAddress + 1] = ((instructionBinary & 0x00FF0000) >> 16);
            m_finalBinary.binaryData[instructionAddress + 2] = ((instructionBinary & 0x0000FF00) >> 8);
            m_finalBinary.binaryData[instructionAddress + 3] = ((instructionBinary & 0x000000FF));

            m_finalBinary.origin[instructionAddress].filePath = m_routineBlocks[i].instructionLines[j].m_originFilePath;
            m_finalBinary.origin[instructionAddress].lineNb = m_routineBlocks[i].instructionLines[j].m_originLineNb;

            instructionAddress += Cpu::INSTRUCTION_SIZE;
        }
    }

    return true;
}

bool Assembler::convertDataToBinary()
{
    for (unsigned int i(0); i < m_definedVars.size(); i++)
    {
        if (m_definedVars[i].type == Token::DataType::SINGLE_VALUE_DEFINED || m_definedVars[i].type == Token::DataType::SINGLE_VALUE_UNDEFINED)
        {
            m_finalBinary.binaryData[m_definedVars[i].range.begin] = m_definedVars[i].values[0].getValue();
        }
        else if (m_definedVars[i].type == Token::DataType::STRING_DEFINED || m_definedVars[i].type == Token::DataType::STRING_UNDEFINED)
        {
            std::string str = m_definedVars[i].values[0].getStr();

            for (unsigned int j(0); j < str.size(); j++)
            {
                m_finalBinary.binaryData[m_definedVars[i].range.begin + j] = str.at(j);
            }
        }
        else // Multiple values
        {
            for (unsigned int j(0); j < m_definedVars[i].values.size(); j++)
            {
                m_finalBinary.binaryData[m_definedVars[i].range.begin + j] = m_definedVars[i].values[j].getValue();
            }
        }
    }

    return true;
}

bool Assembler::saveBinaryToFile()
{
    QString romDirectoryPath(QFileInfo(m_finalFile.m_filePath).path() + "/rom/");
    QDir romDirectory(romDirectoryPath);

    if (!romDirectory.exists())
    {
        romDirectory.mkpath(romDirectoryPath);
    }

    QString filePath = QFileInfo(m_finalFile.m_filePath).path() + "/rom/" + m_finalFile.m_fileName + ".bin";
    QFile file(filePath);

    if (!file.open(QIODevice::WriteOnly))
    {
        m_error.originFilePath = "";
        m_error.originLineNb = 0;
        m_error.type = Token::ErrorType::BIN_FILE_OPEN;
        m_error.additionalInfo = file.errorString().toStdString() + ")";

        logError();
        return false;
    }

    QDataStream out(&file);

    for (unsigned int i(0); i < Eeprom::MEMORY_SIZE; i++)
    {
        out << m_finalBinary.binaryData[i];
    }

    file.flush();
    file.close();

    m_finalBinary.binaryFilePath = QFileInfo(m_finalFile.m_filePath).path() + "/rom/" + m_finalFile.m_fileName + ".bin";

    return true;
}

// -- Utils --
Token::TokenFile* Assembler::findTokenFile(QString fileName)
{
    for (unsigned int i(0); i < m_tokenFiles.size(); i++)
    {
        if (m_tokenFiles[i].m_fileName == fileName)
        {
            return &(m_tokenFiles[i]);
        }
    }

    return nullptr;
}

bool Assembler::variableAddressInferiorComparator(Variable a, Variable b)
{
    return a.range.begin < b.range.begin;
}

bool Assembler::routineAddressInferiorComparator(RoutineBlock a, RoutineBlock b)
{
    return a.range.begin < b.range.begin;
}

bool Assembler::freeMemSpaceAddressInferiorComparator(MemorySpace a, MemorySpace b)
{
    return a.range.begin < b.range.begin;
}

bool Assembler::splitFreeMemorySpace(unsigned int freeMemorySpaceIndex, unsigned int definedVariableIndex)
{
    if (freeMemorySpaceIndex >= m_freeMemorySpaces.size())
        return false;

    if (definedVariableIndex >= m_definedVars.size())
        return false;

    if (m_definedVars[definedVariableIndex].range.begin == m_freeMemorySpaces[freeMemorySpaceIndex].range.begin)
    {
        if (m_definedVars[definedVariableIndex].range.end == m_freeMemorySpaces[freeMemorySpaceIndex].range.end)
        {
            // The data definition occupies all of the free memory space
            m_freeMemorySpaces.erase(m_freeMemorySpaces.begin() + freeMemorySpaceIndex);
        }
        else
        {
            // It starts at the same address as a the free memory space, but leaves space after
            m_freeMemorySpaces[freeMemorySpaceIndex].range.begin = m_definedVars[definedVariableIndex].range.end + 1;
            //m_freeMemorySpaces[freeMemorySpaceIndex].size -= m_definedVars[definedVariableIndex].size;
        }
    }
    else if (m_definedVars[definedVariableIndex].range.end == m_freeMemorySpaces[freeMemorySpaceIndex].range.end)
    {
        // It ends where the free memory space ends, but leaves space before
        m_freeMemorySpaces[freeMemorySpaceIndex].range.end -= m_definedVars[definedVariableIndex].size;
    }
    else
    {
        // Is inside the free memory space without touching the borders (SPLIT)
        MemorySpace newSpace;

        newSpace.range.begin = m_definedVars[definedVariableIndex].range.end + 1;
        newSpace.range.end = m_freeMemorySpaces[freeMemorySpaceIndex].range.end;

        m_freeMemorySpaces.push_back(newSpace);

        m_freeMemorySpaces[freeMemorySpaceIndex].range.end = m_definedVars[definedVariableIndex].range.begin - 1;
    }

    // SORT FREE MEMORY SPACE BY ADDRESS IN ASCENDING ORDER
    std::sort(m_freeMemorySpaces.begin(), m_freeMemorySpaces.end(), freeMemSpaceAddressInferiorComparator);

    return true;
}

bool Assembler::doRangeOverlap(MemoryRange a, MemoryRange b)
{
    if ((a.begin >= b.begin && a.begin <= b.end)
     || (a.end   >= b.begin && a.end   <= b.end))
    {
        return true;
    }

    if ((b.begin >= a.begin && b.begin <= a.end)
     || (b.end   >= a.begin && b.end   <= a.end))
    {
        return true;
    }

    return false;
}

uint32_t Assembler::getBinaryFromTokenLine(Token::TokenLine* line)
{
    uint32_t finalBinary(0x00000000);
    uint8_t byte1(0x00), byte2(0x00), byte3(0x00), byte4(0x00);
    uint8_t opcode(0x00), addrMode(0x00);
    uint8_t r1(0x00), r2(0x00), r3(0x00);
    uint8_t v1(0x00), v2(0x00);
    uint16_t vx(0x0000);
    bool useR3(false), useVx(false);

    // Populate arguments with addressing mode
    switch (line->m_instr.m_addrMode)
    {
    case Cpu::AddressingMode::NONE: // INS
        // No argument to populate
        break;

    case Cpu::AddressingMode::REG: // INS r | INS r, r
        if (line->m_instr.m_opcode == Cpu::InstructionOpcode::DEC || line->m_instr.m_opcode == Cpu::InstructionOpcode::INC
         || line->m_instr.m_opcode == Cpu::InstructionOpcode::NOT || line->m_instr.m_opcode == Cpu::InstructionOpcode::POP
         || line->m_instr.m_opcode == Cpu::InstructionOpcode::PSH || line->m_instr.m_opcode == Cpu::InstructionOpcode::SHL
         || line->m_instr.m_opcode == Cpu::InstructionOpcode::ASR || line->m_instr.m_opcode == Cpu::InstructionOpcode::SHR)
        {
            r1 = (uint8_t)line->m_tokens[1].getRegister();
        }
        else
        {
            r1 = (uint8_t)line->m_tokens[1].getRegister();
            r2 = (uint8_t)line->m_tokens[2].getRegister();
        }
        break;

    case Cpu::AddressingMode::REG_IMM8: // INS r, 0xXX
        r1 = (uint8_t)line->m_tokens[1].getRegister();
        v1 = line->m_tokens[2].getValue();
        break;

    case Cpu::AddressingMode::REG_RAM: // INS r, $0xXXXX
        if (line->m_instr.m_opcode == Cpu::InstructionOpcode::STR) // str $0xXXXX, r | $0xXXXX <- r
        {
            r1 = (uint8_t)line->m_tokens[2].getRegister();
            vx = line->m_tokens[1].getAddress();
        }
        else
        {
            r1 = (uint8_t)line->m_tokens[1].getRegister();
            vx = line->m_tokens[2].getAddress();
        }

        useVx = true;
        break;

    case Cpu::AddressingMode::RAMREG_IMMREG: // INS r, [rr]
        if (line->m_instr.m_opcode == Cpu::InstructionOpcode::STR) // str [rr], r | [rr] <- r
        {
            r1 = (uint8_t)line->m_tokens[1].getConcatRegs().msReg;
            r2 = (uint8_t)line->m_tokens[1].getConcatRegs().lsReg;

            r3 = (uint8_t)line->m_tokens[2].getRegister();
        }
        else
        {
            r1 = (uint8_t)line->m_tokens[2].getConcatRegs().msReg;
            r2 = (uint8_t)line->m_tokens[2].getConcatRegs().lsReg;

            r3 = (uint8_t)line->m_tokens[1].getRegister();
        }

        useR3 = true;
        break;

    case Cpu::AddressingMode::REG16: // INS r, 0xXX
        r1 = (uint8_t)line->m_tokens[1].getConcatRegs().msReg;
        r2 = (uint8_t)line->m_tokens[1].getConcatRegs().lsReg;
        break;

    case Cpu::AddressingMode::IMM16: // INS 0xXXXX
        vx = line->m_tokens[1].getAddress();

        useVx = true;
        break;

    case Cpu::AddressingMode::IMM8: // INS 0xXX
        v1 = line->m_tokens[1].getValue();
        break;
    }

    // Integrate binary data
    opcode = (uint8_t)line->m_instr.m_opcode;
    addrMode = (uint8_t)line->m_instr.m_addrMode;

    byte1 = opcode << 2;
    byte1 += (addrMode & 0xC) >> 2;

    byte2 = (addrMode & 0x3) << 6;
    byte2 += (r1 & 0x7) << 3;
    byte2 += (r2 & 0x7);

    if (useVx)
    {
        byte3 = (vx & 0xFF00) >> 8;
        byte4 = (vx & 0x00FF);
    }
    else
    {
        if (useR3)
            byte3 = (r3 & 0x7);
        else
            byte3 = v1;

        byte4 = v2;
    }

    finalBinary = ((uint32_t)byte1 << 24);
    finalBinary += ((uint32_t)byte2 << 16);
    finalBinary += ((uint32_t)byte3 << 8);
    finalBinary += byte4;

    return finalBinary;
}
