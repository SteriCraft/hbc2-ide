#include "disassembler.h"
#include "computerDetails.h"
#include <QGuiApplication>

DisassemblyViewer* DisassemblyViewer::m_singleton = nullptr;

DisassemblyViewer* DisassemblyViewer::getInstance(QFont font, ConfigManager *configManager, QWidget *parent)
{
    if (m_singleton == nullptr)
    {
        m_singleton = new DisassemblyViewer(font, configManager, parent);
    }

    return m_singleton;
}

void DisassemblyViewer::update(const QByteArray ramData, Console *consoleOutput)
{
    if (m_singleton != nullptr)
    {
        m_singleton->disassemble(ramData);
        consoleOutput->log("RAM binary content disassembled");
    }
}

void DisassemblyViewer::highlightInstruction(Word programCounter)
{
    if (m_singleton != nullptr)
    {
        m_singleton->highlightAddress(programCounter);
    }
}

bool DisassemblyViewer::isOpen()
{
    if (m_singleton != nullptr)
    {
        return m_singleton->isVisible();
    }

    return false;
}

void DisassemblyViewer::close()
{
    if (m_singleton != nullptr)
    {
        m_singleton->hide();
        delete m_singleton;
        m_singleton = nullptr;
    }
}

DisassemblyViewer::~DisassemblyViewer()
{
    m_singleton = nullptr;
}

// PROTECTED
void DisassemblyViewer::showEvent(QShowEvent* event)
{
    QWidget::showEvent(event);
    setPosition();
}

// PRIVATE
DisassemblyViewer::DisassemblyViewer(QFont font, ConfigManager *configManager, QWidget *parent) : QDialog(parent)
{
    resize(WINDOW_WIDTH, WINDOW_HEIGHT);
    setPosition();

    setWindowTitle(tr("Disassembler"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    // Widgets
    m_disassembledCode = new CustomizedCodeEditor(font, configManager, this);
    m_disassembledCode->setReadOnly(true);

    QPushButton *closeButton = new QPushButton(tr("Close"), this);

    // Layout
    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(m_disassembledCode);
    mainLayout->addWidget(closeButton);

    setLayout(mainLayout);

    // Connections
    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));
}

void DisassemblyViewer::setPosition()
{
    QSize screenSize;
    screenSize = QGuiApplication::primaryScreen()->geometry().size();

    QPoint bottomLeftMainWinPos;
    bottomLeftMainWinPos.setX(parentWidget()->geometry().left());
    bottomLeftMainWinPos.setY(parentWidget()->geometry().bottom());

    QPoint dialogPos;

    if (bottomLeftMainWinPos.x() < 1)
        dialogPos.setX(1);
    else if (bottomLeftMainWinPos.x() > screenSize.width())
        dialogPos.setX(screenSize.width());
    else
        dialogPos.setX(bottomLeftMainWinPos.x() - size().width());

    if (bottomLeftMainWinPos.y() < 1)
        dialogPos.setY(1);
    else if (bottomLeftMainWinPos.y() > screenSize.height())
        dialogPos.setY(screenSize.height());
    else
        dialogPos.setY(bottomLeftMainWinPos.y());

    move(dialogPos);
}

void DisassemblyViewer::disassemble(const QByteArray ramData)
{
    std::vector<QString> lines;

    Disassembler::Label startLabel;
    startLabel.address = Cpu::PROGRAM_START_ADDRESS;
    startLabel.name = "_start";

    m_labelsList.clear();
    m_labelsList.push_back(startLabel);

    m_disassembledCode->clear();
    m_startAddressLineNumber = 0;

    // Program memory
    for (unsigned int i(Cpu::PROGRAM_START_ADDRESS); i < Ram::MEMORY_SIZE; i += Cpu::INSTRUCTION_SIZE)
    {
        m_instruction =  (((Dword)ramData[i]) << 24) & 0xFF000000;
        m_instruction += (((Dword)ramData[i + 1]) << 16) & 0x00FF0000;
        m_instruction += (((Dword)ramData[i + 2]) << 8) & 0x0000FF00;
        m_instruction += ramData[i + 3] & 0x000000FF;

        decodeInstruction();
        convertInstructionToQString();

        lines.push_back("                " + QString("0x" + QString::number(i, 16) + "\t" + m_disassembledInstructionStr));
    }

    // Labels
    for (int i(m_labelsList.size() - 1); i >= 0; i--)
    {
        lines[((m_labelsList[i].address - Cpu::PROGRAM_START_ADDRESS) / Cpu::INSTRUCTION_SIZE)].replace(0, m_labelsList[i].name.size() + 1, ":" + m_labelsList[i].name);
    }

    // Variables
    lines.insert(lines.begin(),     ";========== VARIABLES =========");
    lines.insert(lines.begin() + 1, "; Current values");
    lines.insert(lines.begin() + 2, ";==============================");
    lines.insert(lines.begin() + 3, "");
    lines.insert(lines.begin() + 4, "");
    lines.insert(lines.begin() + 5, ";======= PROGRAM MEMORY =======");

    for (unsigned int i(0); i < m_variablesList.size(); i++)
    {
        QString variableStr;

        variableStr = ".data ";
        variableStr += m_variablesList[i].name;
        variableStr += " $0x" + QString::number(m_variablesList[i].address, 16);
        variableStr += " 0x" + QString::number(ramData[m_variablesList[i].address], 16);

        lines.insert(lines.begin() + 2 + i, variableStr);
    }

    // IVT content
    lines.insert(lines.begin(),     ";=== INTERRUPT VECTOR TABLE ===");
    lines.insert(lines.begin() + 1, ";==============================");
    lines.insert(lines.begin() + 2, "");
    lines.insert(lines.begin() + 3, "");

    unsigned int vectorCount(0);
    for (unsigned int i(Cpu::IVT_START_ADDRESS); i < Cpu::IVT_START_ADDRESS + Cpu::IVT_SIZE; i += 2)
    {
        QString vectorStr;
        Word vectorAddress;
        vectorAddress =  (((Word)ramData[i]) << 8) & 0xFF00;
        vectorAddress += ramData[i + 1] & 0x00FF;

        if (vectorAddress != 0x0000)
        {
            vectorCount++;

            vectorStr = ".data vector" + QString::number(vectorCount);
            vectorStr += " $0x" + QString::number(i, 16);
            vectorStr += " 0x" + QString::number(vectorAddress, 16);

            lines.insert(lines.begin() + 1, vectorStr);

            m_startAddressLineNumber++;
        }
    }
    m_startAddressLineNumber += m_variablesList.size() + 10;

    // Final dump
    for (unsigned int i(0); i < lines.size(); i++)
    {
        m_disassembledCode->appendPlainText(lines[i]);
    }

    QTextCursor cursor = m_disassembledCode->textCursor();
    cursor.movePosition(QTextCursor::Start);
    m_disassembledCode->setTextCursor(cursor);
}

void DisassemblyViewer::decodeInstruction()
{
    m_decodedInstruction.opcode = (Cpu::InstructionOpcode)((m_instruction & Cpu::OPCODE_MASK) >> 26);
    m_decodedInstruction.addressingMode = (Cpu::AddressingMode)((m_instruction & Cpu::ADDRMODE_MASK) >> 22);

    m_decodedInstruction.r1 = (Cpu::Register)((m_instruction & Cpu::R1_MASK) >> 19);
    m_decodedInstruction.r2 = (Cpu::Register)((m_instruction & Cpu::R2_MASK) >> 16);
    m_decodedInstruction.r3 = (Cpu::Register)((m_instruction & Cpu::R3_MASK) >> 8);

    m_decodedInstruction.v1 = (m_instruction & Cpu::V1_MASK) >> 8;
    m_decodedInstruction.v2 = m_instruction & Cpu::V2_MASK;
    m_decodedInstruction.vX = m_instruction & Cpu::VX_MASK;
}

void DisassemblyViewer::convertInstructionToQString()
{
    Disassembler::Label newLabel;
    Disassembler::Variable newVariable;
    QString opcodeStr("nop");
    QString argsStr("");
    bool valid(false);

    if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::ADC || m_decodedInstruction.opcode == Cpu::InstructionOpcode::ADD
     || m_decodedInstruction.opcode == Cpu::InstructionOpcode::AND || m_decodedInstruction.opcode == Cpu::InstructionOpcode::OR
     || m_decodedInstruction.opcode == Cpu::InstructionOpcode::ADD || m_decodedInstruction.opcode == Cpu::InstructionOpcode::SUB
     || m_decodedInstruction.opcode == Cpu::InstructionOpcode::SBB || m_decodedInstruction.opcode == Cpu::InstructionOpcode::XOR)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG)
        {
            if ((int)m_decodedInstruction.r1 < Cpu ::REGISTERS_NB && (int)m_decodedInstruction.r2 < Cpu ::REGISTERS_NB)
            {
                argsStr =  Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                argsStr += ", ";
                argsStr += Cpu::regStrArr[(int)m_decodedInstruction.r2].c_str();
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG_IMM8)
        {
            if ((int)m_decodedInstruction.r1 < Cpu ::REGISTERS_NB)
            {
                argsStr =  Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                argsStr += ", 0x";
                argsStr += QString::number(m_decodedInstruction.v1, 16);
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG_RAM)
        {
            if ((int)m_decodedInstruction.r1 < Cpu ::REGISTERS_NB)
            {
                argsStr =  Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();

                newVariable.address = m_decodedInstruction.vX;
                newVariable.name = "var" + QString::number(m_variablesList.size() + 1);
                int variableIndex(getVariableIndex(newVariable));

                if (variableIndex == -1)
                {
                    m_variablesList.push_back(newVariable);
                    argsStr += ", " + newLabel.name;
                }
                else
                {
                    argsStr += ", " + m_variablesList[variableIndex].name;
                }

                valid = true;
            }
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::CLC || m_decodedInstruction.opcode == Cpu::InstructionOpcode::CLE
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::CLI || m_decodedInstruction.opcode == Cpu::InstructionOpcode::CLN
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::CLS || m_decodedInstruction.opcode == Cpu::InstructionOpcode::CLZ
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::CLF || m_decodedInstruction.opcode == Cpu::InstructionOpcode::HLT
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::IRT || m_decodedInstruction.opcode == Cpu::InstructionOpcode::RET
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::STC || m_decodedInstruction.opcode == Cpu::InstructionOpcode::STE
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::STI || m_decodedInstruction.opcode == Cpu::InstructionOpcode::STN
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::STS || m_decodedInstruction.opcode == Cpu::InstructionOpcode::STZ
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::STF)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::NONE)
        {
            valid = true;
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::IN  || m_decodedInstruction.opcode == Cpu::InstructionOpcode::OUT
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::POP || m_decodedInstruction.opcode == Cpu::InstructionOpcode::PSH
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::SHL || m_decodedInstruction.opcode == Cpu::InstructionOpcode::ASR
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::SHR)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB)
            {
                argsStr = Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                valid = true;
            }
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::CAL || m_decodedInstruction.opcode == Cpu::InstructionOpcode::JMC
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::JME || m_decodedInstruction.opcode == Cpu::InstructionOpcode::JMN
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::JMP || m_decodedInstruction.opcode == Cpu::InstructionOpcode::JMS
          || m_decodedInstruction.opcode == Cpu::InstructionOpcode::JMZ || m_decodedInstruction.opcode == Cpu::InstructionOpcode::JMF)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG16)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB && (int)m_decodedInstruction.r2 < Cpu::REGISTERS_NB)
            {
                argsStr =  "[";
                argsStr += Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                argsStr += Cpu::regStrArr[(int)m_decodedInstruction.r2].c_str();
                argsStr += "]";
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::IMM16)
        {
            // if address < 0x300 -> pas de label
            if (m_decodedInstruction.vX >= Cpu::PROGRAM_START_ADDRESS)
            {
                newLabel.address = m_decodedInstruction.vX;
                newLabel.name = "_location" + QString::number(m_labelsList.size());
                int labelIndex(getLabelIndex(newLabel));

                if (labelIndex == -1)
                {
                    m_labelsList.push_back(newLabel);
                    argsStr = newLabel.name;
                }
                else
                {
                    argsStr = m_labelsList[labelIndex].name;
                }
            }
            else
            {
                argsStr = "0x";
                argsStr += QString::number(m_decodedInstruction.vX, 16);
            }

            valid = true;
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::DEC || m_decodedInstruction.opcode == Cpu::InstructionOpcode::INC)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB)
            {
                argsStr = Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG16)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB && (int)m_decodedInstruction.r2 < Cpu::REGISTERS_NB)
            {
                argsStr =  "[";
                argsStr += Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                argsStr += Cpu::regStrArr[(int)m_decodedInstruction.r2].c_str();
                argsStr += "]";
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::IMM16)
        {
            newVariable.address = m_decodedInstruction.vX;
            newVariable.name = "var" + QString::number(m_variablesList.size() + 1);
            int variableIndex(getVariableIndex(newVariable));

            if (variableIndex == -1)
            {
                m_variablesList.push_back(newVariable);
                argsStr = newLabel.name;
            }
            else
            {
                argsStr = m_variablesList[variableIndex].name;
            }

            valid = true;
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::STR || m_decodedInstruction.opcode == Cpu::InstructionOpcode::LOD)
    {
        QString arg1, arg2;
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG_RAM)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB)
            {
                arg1 = Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();

                newVariable.address = m_decodedInstruction.vX;
                newVariable.name = "var" + QString::number(m_variablesList.size() + 1);
                int variableIndex(getVariableIndex(newVariable));

                if (variableIndex == -1)
                {
                    m_variablesList.push_back(newVariable);
                    arg2 = newLabel.name;
                }
                else
                {
                    arg2 = m_variablesList[variableIndex].name;
                }

                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::RAMREG_IMMREG)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB && (int)m_decodedInstruction.r2 < Cpu::REGISTERS_NB && (int)m_decodedInstruction.r3 < Cpu::REGISTERS_NB)
            {
                arg1 = Cpu::regStrArr[(int)m_decodedInstruction.r3].c_str();
                arg2 += "[";
                arg2 += Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                arg2 += Cpu::regStrArr[(int)m_decodedInstruction.r2].c_str();
                arg2 += "]";
                valid = true;
            }
        }

        if (valid)
        {
            if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::STR)
            {
                argsStr = arg2 + ", " + arg1;
            }
            else
            {
                argsStr = arg1 + ", " + arg2;
            }
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::CMP)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB)
            {
                argsStr = Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG_IMM8)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB)
            {
                argsStr =  Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                argsStr += ", 0x";
                argsStr += QString::number(m_decodedInstruction.v1, 16);
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::RAMREG_IMMREG)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB && (int)m_decodedInstruction.r2 < Cpu::REGISTERS_NB && (int)m_decodedInstruction.r3 < Cpu::REGISTERS_NB)
            {
                argsStr =  Cpu::regStrArr[(int)m_decodedInstruction.r3].c_str();
                argsStr += ", [";
                argsStr += Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                argsStr += Cpu::regStrArr[(int)m_decodedInstruction.r2].c_str();
                argsStr += "]";
                valid = true;
            }
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::MOV)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB)
            {
                argsStr = Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG_IMM8)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB)
            {
                argsStr =  Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                argsStr += ", 0x";
                argsStr += QString::number(m_decodedInstruction.v1, 16);
                valid = true;
            }
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::NOT)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::REG)
        {
            if ((int)m_decodedInstruction.r1 < Cpu::REGISTERS_NB)
            {
                argsStr = Cpu::regStrArr[(int)m_decodedInstruction.r1].c_str();
                valid = true;
            }
        }
        else if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::IMM16)
        {
            newVariable.address = m_decodedInstruction.vX;
            newVariable.name = "var" + QString::number(m_variablesList.size() + 1);
            int variableIndex(getVariableIndex(newVariable));

            if (variableIndex == -1)
            {
                m_variablesList.push_back(newVariable);
                argsStr = newLabel.name;
            }
            else
            {
                argsStr = m_variablesList[variableIndex].name;
            }

            valid = true;
        }
    }
    else if (m_decodedInstruction.opcode == Cpu::InstructionOpcode::INT)
    {
        if (m_decodedInstruction.addressingMode == Cpu::AddressingMode::IMM8)
        {
            argsStr =  "0x";
            argsStr += QString::number(m_decodedInstruction.v1, 16);
            valid = true;
        }
    }

    if (valid)
    {
        opcodeStr = Cpu::instrStrArr[(int)m_decodedInstruction.opcode].c_str();
        m_disassembledInstructionStr = opcodeStr + " " + argsStr;
    }
    else
    {
        m_disassembledInstructionStr = "nop";
    }
}

int DisassemblyViewer::getLabelIndex(Disassembler::Label newLabel)
{
    for (unsigned int i(0); i < m_labelsList.size(); i++)
    {
        if (m_labelsList[i].address == newLabel.address)
            return i;
    }

    return -1;
}

int DisassemblyViewer::getVariableIndex(Disassembler::Variable newVariable)
{
    for (unsigned int i(0); i < m_variablesList.size(); i++)
    {
        if (m_variablesList[i].address == newVariable.address)
            return i;
    }

    return -1;
}

void DisassemblyViewer::highlightAddress(Word programCounter)
{
    m_disassembledCode->highlightLine(m_startAddressLineNumber + (programCounter - Cpu::PROGRAM_START_ADDRESS) / Cpu::INSTRUCTION_SIZE);
}
