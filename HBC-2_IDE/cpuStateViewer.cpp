#include "cpuStateViewer.h"

#include <QHeaderView>

CpuStateViewer* CpuStateViewer::m_singleton = nullptr;
CpuStatus CpuStateViewer::m_savedState;

CpuStateViewer* CpuStateViewer::getInstance(QWidget *parent)
{
    if (m_singleton == nullptr)
    {
        m_singleton = new CpuStateViewer(parent);
    }

    return m_singleton;
}

void CpuStateViewer::update(CpuStatus status)
{
    m_savedState = status;

    if (m_singleton != nullptr)
    {
        m_singleton->updateStatus();
    }
}

QString CpuStateViewer::byte2QString(Byte value, Base base, unsigned int significantDigits)
{
    QString result("");

    switch (base)
    {
        case Base::BINARY:
            for (unsigned int i(0x80); i > 0x01; i /= 2)
            {
                if (value < i)
                    result += "0";
            }

            result += QString::number(value, 2);
            break;

        case Base::DECIMAL:
            result = QString::number(value, 10);
            break;

        case Base::HEXADECIMAL:
            if (value < 0x10)
                result += "0";

            result += QString::number(value, 16).toUpper();
            break;
    }

    if (significantDigits > 0)
    {
        result = result.right(significantDigits);
    }

    return ((base == Base::HEXADECIMAL) ? "0x" : "") + result;
}

QString CpuStateViewer::word2QString(Word value, Base base, unsigned int significantDigits)
{
    QString result("");

    switch (base)
    {
        case Base::BINARY:
            for (unsigned int i(0x8000); i > 0x01; i /= 2)
            {
                if (value < i)
                    result += "0";
            }

            result += QString::number(value, 2);
            break;

        case Base::DECIMAL:
            result = QString::number(value, 10);
            break;

        case Base::HEXADECIMAL:
            for (unsigned int i(0x1000); i > 0x1; i /= 16)
            {
                if (value < i)
                    result += "0";
            }

            result += QString::number(value, 16).toUpper();
            break;
    }

    if (significantDigits > 0)
    {
        result = result.right(significantDigits);
    }

    return ((base == Base::HEXADECIMAL) ? "0x" : "") + result;
}

QString CpuStateViewer::dWord2QString(Dword value, Base base, unsigned int significantDigits)
{
    QString result("");

    switch (base)
    {
        case Base::BINARY:
            for (unsigned int i(0x80000000); i > 0x01; i /= 2)
            {
                if (value < i)
                    result += "0";
            }

            result += QString::number(value, 2);
            break;

        case Base::DECIMAL:
            result = QString::number(value, 10);
            break;

        case Base::HEXADECIMAL:
            for (unsigned int i(0x10000000); i > 0x1; i /= 16)
            {
                if (value < i)
                    result += "0";
            }

            result += QString::number(value, 16).toUpper();
            break;
    }

    if (significantDigits > 0)
    {
        result = result.right(significantDigits);
    }

    return ((base == Base::HEXADECIMAL) ? "0x" : "") + result;
}

void CpuStateViewer::close()
{
    if (m_singleton != nullptr)
    {
        m_singleton->hide();
        delete m_singleton;
        m_singleton = nullptr;
    }
}

// PRIVATE SLOTS
void CpuStateViewer::switchToBinaryBase()
{
    m_base = Base::BINARY;
    changeValueBase();
}

void CpuStateViewer::switchToDecimalBase()
{
    m_base = Base::DECIMAL;
    changeValueBase();
}

void CpuStateViewer::switchToHexadecimalBase()
{
    m_base = Base::HEXADECIMAL;
    changeValueBase();
}


// PRIVATE
CpuStateViewer::CpuStateViewer(QWidget *parent) : QDialog(parent)
{
    QStringList header;

    setWindowTitle(tr("CPU State"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    setFixedWidth(CPU_STATE_VIEWER_WIDTH);

    m_base = Base::HEXADECIMAL;


    // ==== STATE GROUP BOX (VLayout) ====
    QGroupBox *stateGroupBox = new QGroupBox(tr("Status"), this);
    QVBoxLayout *stateGroupLayout = new QVBoxLayout;

    m_lastStateLabel = new QLabel(tr("Last dump before reinitialization"), this);
    m_lastStateLabel->setStyleSheet("color: red;");
    m_lastStateLabel->setAlignment(Qt::AlignHCenter);

    m_stateLineEdit = new QLineEdit(this);
    m_stateLineEdit->setReadOnly(true);
    m_stateLineEdit->setFixedWidth(STATE_WIDTH);
    m_stateLineEdit->setAlignment(Qt::AlignHCenter);

    m_interruptReadyLineEdit = new QLineEdit(this);
    m_interruptReadyLineEdit->setReadOnly(true);
    m_interruptReadyLineEdit->setFixedWidth(STATE_WIDTH);
    m_interruptReadyLineEdit->setAlignment(Qt::AlignHCenter);

    QHBoxLayout *programCounterLayout = new QHBoxLayout;
    QLabel *programCounterLabel = new QLabel(tr("Program counter"), this);
    m_programCounterLineEdit = new QLineEdit(this);
    m_programCounterLineEdit->setReadOnly(true);
    m_programCounterLineEdit->setFixedWidth(INSTRUCTION_WIDTH);
    m_programCounterLineEdit->setAlignment(Qt::AlignHCenter);
    programCounterLayout->addWidget(programCounterLabel);
    programCounterLayout->addWidget(m_programCounterLineEdit);

    QHBoxLayout *instructionRegisterLayout = new QHBoxLayout;
    QLabel *instructionRegisterLabel = new QLabel(tr("Instruction register"), this);
    m_instructionRegisterLineEdit = new QLineEdit(this);
    m_instructionRegisterLineEdit->setReadOnly(true);
    m_instructionRegisterLineEdit->setFixedWidth(INSTRUCTION_WIDTH);
    m_instructionRegisterLineEdit->setAlignment(Qt::AlignHCenter);
    instructionRegisterLayout->addWidget(instructionRegisterLabel);
    instructionRegisterLayout->addWidget(m_instructionRegisterLineEdit);

    QHBoxLayout *adressingModeLayout = new QHBoxLayout;
    QLabel *adressingModeLabel = new QLabel(tr("Addressing mode"), this);
    m_addressingModeLineEdit = new QLineEdit(this);
    m_addressingModeLineEdit->setReadOnly(true);
    m_addressingModeLineEdit->setFixedWidth(INSTRUCTION_WIDTH);
    m_addressingModeLineEdit->setAlignment(Qt::AlignHCenter);
    adressingModeLayout->addWidget(adressingModeLabel);
    adressingModeLayout->addWidget(m_addressingModeLineEdit);

    m_decodedInstructionTable = new QTableWidget(1, 8, this);
    header << "Opcode" << "AddrMode" << "R1" << "R2" << "R3" << "V1" << "V2" << "Vx";
    m_decodedInstructionTable->setHorizontalHeaderLabels(header);
    m_decodedInstructionTable->setEditTriggers(QAbstractItemView::NoEditTriggers);
    m_decodedInstructionTable->setFocusPolicy(Qt::NoFocus);
    m_decodedInstructionTable->setSelectionMode(QAbstractItemView::NoSelection);
    m_decodedInstructionTable->verticalHeader()->setVisible(false);
    m_decodedInstructionTable->horizontalHeader()->setSectionResizeMode(QHeaderView::Fixed);
    m_decodedInstructionTable->setFixedSize(DECODED_INSTRUCTION_TABLE_WIDTH, TABLE_HEIGHT);

    m_opcodeTableItem = new QTableWidgetItem("0x00");
    m_opcodeTableItem->setTextAlignment(Qt::AlignCenter);
    m_decodedInstructionTable->setItem(0, 0, m_opcodeTableItem);
    m_decodedInstructionTable->setColumnWidth(0, WORD_ITEM_WIDTH);

    m_addrModeTableItem = new QTableWidgetItem("0x0");
    m_addrModeTableItem->setTextAlignment(Qt::AlignCenter);
    m_decodedInstructionTable->setItem(0, 1, m_addrModeTableItem);
    m_decodedInstructionTable->setColumnWidth(1, ADDR_MODE_TABLE_ITEM_WIDTH);

    m_r1TableItem = new QTableWidgetItem("0x0");
    m_r1TableItem->setTextAlignment(Qt::AlignCenter);
    m_decodedInstructionTable->setItem(0, 2, m_r1TableItem);
    m_decodedInstructionTable->setColumnWidth(2, REG_ITEM_WIDTH);

    m_r2TableItem = new QTableWidgetItem("0x0");
    m_r2TableItem->setTextAlignment(Qt::AlignCenter);
    m_decodedInstructionTable->setItem(0, 3, m_r2TableItem);
    m_decodedInstructionTable->setColumnWidth(3, REG_ITEM_WIDTH);

    m_r3TableItem = new QTableWidgetItem("0x0");
    m_r3TableItem->setTextAlignment(Qt::AlignCenter);
    m_decodedInstructionTable->setItem(0, 4, m_r3TableItem);
    m_decodedInstructionTable->setColumnWidth(4, REG_ITEM_WIDTH);

    m_v1TableItem = new QTableWidgetItem("0x00");
    m_v1TableItem->setTextAlignment(Qt::AlignCenter);
    m_decodedInstructionTable->setItem(0, 5, m_v1TableItem);
    m_decodedInstructionTable->setColumnWidth(5, REG_ITEM_WIDTH);

    m_v2TableItem = new QTableWidgetItem("0x00");
    m_v2TableItem->setTextAlignment(Qt::AlignCenter);
    m_decodedInstructionTable->setItem(0, 6, m_v2TableItem);
    m_decodedInstructionTable->setColumnWidth(6, REG_ITEM_WIDTH);

    m_vXTableItem = new QTableWidgetItem("0x0000");
    m_vXTableItem->setTextAlignment(Qt::AlignCenter);
    m_decodedInstructionTable->setItem(0, 7, m_vXTableItem);
    m_decodedInstructionTable->setColumnWidth(7, WORD_ITEM_WIDTH);

    stateGroupLayout->addWidget(m_lastStateLabel);
    stateGroupLayout->addWidget(m_stateLineEdit);
    stateGroupLayout->setAlignment(m_stateLineEdit, Qt::AlignHCenter);
    stateGroupLayout->addWidget(m_interruptReadyLineEdit);
    stateGroupLayout->setAlignment(m_interruptReadyLineEdit, Qt::AlignHCenter);
    stateGroupLayout->addLayout(programCounterLayout);
    stateGroupLayout->addLayout(instructionRegisterLayout);
    stateGroupLayout->addLayout(adressingModeLayout);
    stateGroupLayout->addWidget(m_decodedInstructionTable);
    stateGroupLayout->setAlignment(m_decodedInstructionTable, Qt::AlignHCenter);
    stateGroupBox->setLayout(stateGroupLayout);
    stateGroupBox->setAlignment(Qt::AlignHCenter);
    // ===================================

    // ==== REGISTERS GROUP BOX (VLayout) ====
    QGroupBox *registersGroupBox = new QGroupBox(tr("Registers"), this);
    QVBoxLayout *registersGroupLayout = new QVBoxLayout;

    QLabel *flagsLabel = new QLabel(tr("Flags"), this);
    QTableWidget *m_flagsTable = new QTableWidget(1, 8, this);
    header.clear();
    header << "C" << "E" << "I" << "N" << "S" << "Z" << "F" << "H";
    m_flagsTable->setHorizontalHeaderLabels(header);
    m_flagsTable->setEditTriggers(QAbstractItemView::NoEditTriggers);
    m_flagsTable->setFocusPolicy(Qt::NoFocus);
    m_flagsTable->setSelectionMode(QAbstractItemView::NoSelection);
    m_flagsTable->verticalHeader()->setVisible(false);
    m_flagsTable->horizontalHeader()->setSectionResizeMode(QHeaderView::Fixed);
    m_flagsTable->setFixedSize(FLAGS_TABLE_WIDTH, TABLE_HEIGHT);
    for (unsigned int i(0); i < Cpu::FLAGS_NB; i++)
    {
        m_flagsTableItem.push_back(new QTableWidgetItem("0"));
        m_flagsTableItem.back()->setTextAlignment(Qt::AlignCenter);
        m_flagsTable->setItem(0, i, m_flagsTableItem.back());
        m_flagsTable->setColumnWidth(i, 10);
    }

    QHBoxLayout *mainRegistersLayout = new QHBoxLayout;
        QVBoxLayout *registerADLayout = new QVBoxLayout;

            QHBoxLayout *registerALayout = new QHBoxLayout;
            QLabel *registerALabel = new QLabel(tr("A"), this);
            m_registersLineEdits.push_back(new QLineEdit(this));
            m_registersLineEdits.back()->setReadOnly(true);
            m_registersLineEdits.back()->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registersLineEdits.back()->setAlignment(Qt::AlignHCenter);
            registerALayout->addWidget(m_registersLineEdits.back());
            registerALayout->addWidget(registerALabel);
            registerALayout->setAlignment(m_registersLineEdits.back(), Qt::AlignHCenter);
            registerALayout->setAlignment(registerALabel, Qt::AlignHCenter);

            QHBoxLayout *registerBLayout = new QHBoxLayout;
            QLabel *registerBLabel = new QLabel(tr("B"), this);
            m_registersLineEdits.push_back(new QLineEdit(this));
            m_registersLineEdits.back()->setReadOnly(true);
            m_registersLineEdits.back()->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registersLineEdits.back()->setAlignment(Qt::AlignHCenter);
            registerBLayout->addWidget(m_registersLineEdits.back());
            registerBLayout->addWidget(registerBLabel);
            registerBLayout->setAlignment(m_registersLineEdits.back(), Qt::AlignHCenter);
            registerBLayout->setAlignment(registerBLabel, Qt::AlignHCenter);

            QHBoxLayout *registerCLayout = new QHBoxLayout;
            QLabel *registerCLabel = new QLabel(tr("C"), this);
            m_registersLineEdits.push_back(new QLineEdit(this));
            m_registersLineEdits.back()->setReadOnly(true);
            m_registersLineEdits.back()->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registersLineEdits.back()->setAlignment(Qt::AlignHCenter);
            registerCLayout->addWidget(m_registersLineEdits.back());
            registerCLayout->addWidget(registerCLabel);
            registerCLayout->setAlignment(m_registersLineEdits.back(), Qt::AlignHCenter);
            registerCLayout->setAlignment(registerCLabel, Qt::AlignHCenter);

            QHBoxLayout *registerDLayout = new QHBoxLayout;
            QLabel *registerDLabel = new QLabel(tr("D"), this);
            m_registersLineEdits.push_back(new QLineEdit(this));
            m_registersLineEdits.back()->setReadOnly(true);
            m_registersLineEdits.back()->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registersLineEdits.back()->setAlignment(Qt::AlignHCenter);
            registerDLayout->addWidget(m_registersLineEdits.back());
            registerDLayout->addWidget(registerDLabel);
            registerDLayout->setAlignment(m_registersLineEdits.back(), Qt::AlignHCenter);
            registerDLayout->setAlignment(registerDLabel, Qt::AlignHCenter);

        registerADLayout->addLayout(registerALayout);
        registerADLayout->addLayout(registerBLayout);
        registerADLayout->addLayout(registerCLayout);
        registerADLayout->addLayout(registerDLayout);

        QVBoxLayout *registerIXLayout = new QVBoxLayout;

            QHBoxLayout *registerILayout = new QHBoxLayout;
            QLabel *registerILabel = new QLabel(tr("I"), this);
            m_registersLineEdits.push_back(new QLineEdit(this));
            m_registersLineEdits.back()->setReadOnly(true);
            m_registersLineEdits.back()->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registersLineEdits.back()->setAlignment(Qt::AlignHCenter);
            registerILayout->addWidget(registerILabel);
            registerILayout->addWidget(m_registersLineEdits.back());
            registerILayout->setAlignment(registerILabel, Qt::AlignHCenter);
            registerILayout->setAlignment(m_registersLineEdits.back(), Qt::AlignHCenter);

            QHBoxLayout *registerJLayout = new QHBoxLayout;
            QLabel *registerJLabel = new QLabel(tr("J"), this);
            m_registersLineEdits.push_back(new QLineEdit(this));
            m_registersLineEdits.back()->setReadOnly(true);
            m_registersLineEdits.back()->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registersLineEdits.back()->setAlignment(Qt::AlignHCenter);
            registerJLayout->addWidget(registerJLabel);
            registerJLayout->addWidget(m_registersLineEdits.back());
            registerJLayout->setAlignment(registerJLabel, Qt::AlignHCenter);
            registerJLayout->setAlignment(m_registersLineEdits.back(), Qt::AlignHCenter);

            QHBoxLayout *registerXLayout = new QHBoxLayout;
            QLabel *registerXLabel = new QLabel(tr("X"), this);
            m_registersLineEdits.push_back(new QLineEdit(this));
            m_registersLineEdits.back()->setReadOnly(true);
            m_registersLineEdits.back()->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registersLineEdits.back()->setAlignment(Qt::AlignHCenter);
            registerXLayout->addWidget(registerXLabel);
            registerXLayout->addWidget(m_registersLineEdits.back());
            registerXLayout->setAlignment(registerXLabel, Qt::AlignHCenter);
            registerXLayout->setAlignment(m_registersLineEdits.back(), Qt::AlignHCenter);

            QHBoxLayout *registerYLayout = new QHBoxLayout;
            QLabel *registerYLabel = new QLabel(tr("Y"), this);
            m_registersLineEdits.push_back(new QLineEdit(this));
            m_registersLineEdits.back()->setReadOnly(true);
            m_registersLineEdits.back()->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registersLineEdits.back()->setAlignment(Qt::AlignHCenter);
            registerYLayout->addWidget(registerYLabel);
            registerYLayout->addWidget(m_registersLineEdits.back());
            registerYLayout->setAlignment(registerYLabel, Qt::AlignHCenter);
            registerYLayout->setAlignment(m_registersLineEdits.back(), Qt::AlignHCenter);

        registerIXLayout->addLayout(registerILayout);
        registerIXLayout->addLayout(registerJLayout);
        registerIXLayout->addLayout(registerXLayout);
        registerIXLayout->addLayout(registerYLayout);

    mainRegistersLayout->addLayout(registerADLayout);
    mainRegistersLayout->addLayout(registerIXLayout);

    registersGroupLayout->addWidget(flagsLabel);
    registersGroupLayout->setAlignment(flagsLabel, Qt::AlignHCenter);
    registersGroupLayout->addWidget(m_flagsTable);
    registersGroupLayout->setAlignment(m_flagsTable, Qt::AlignHCenter);
    registersGroupLayout->addLayout(mainRegistersLayout);
    registersGroupBox->setLayout(registersGroupLayout);
    registersGroupBox->setAlignment(Qt::AlignHCenter);
    // =======================================

    // ==== STACK GROUP BOX (HLayout) ====
    QGroupBox *stackGroupBox = new QGroupBox(tr("Stack"), this);
    QHBoxLayout *stackGroupLayout = new QHBoxLayout;

    QLabel *stackPointerLabel = new QLabel(tr("Stack pointer"), this);
    m_stackPointerLineEdit = new QLineEdit(this);
    m_stackPointerLineEdit->setReadOnly(true);
    m_stackPointerLineEdit->setFixedWidth(INSTRUCTION_WIDTH);
    m_stackPointerLineEdit->setAlignment(Qt::AlignHCenter);

    stackGroupLayout->addWidget(stackPointerLabel);
    stackGroupLayout->addWidget(m_stackPointerLineEdit);
    stackGroupBox->setLayout(stackGroupLayout);
    stackGroupBox->setAlignment(Qt::AlignHCenter);
    // ===================================

    // ==== BUSES BOX (HLayout) ====
    QGroupBox *busesGroupBox = new QGroupBox(tr("Buses"), this);
    QVBoxLayout *busesGroupLayout = new QVBoxLayout;

    QHBoxLayout *addressBusLayout = new QHBoxLayout;
    QLabel *addressBusLabel = new QLabel(tr("Address bus"), this);
    m_addressBusLineEdit = new QLineEdit(this);
    m_addressBusLineEdit->setReadOnly(true);
    m_addressBusLineEdit->setFixedWidth(INSTRUCTION_WIDTH);
    m_addressBusLineEdit->setAlignment(Qt::AlignHCenter);
    addressBusLayout->addWidget(addressBusLabel);
    addressBusLayout->addWidget(m_addressBusLineEdit);

    QHBoxLayout *dataBusLayout = new QHBoxLayout;
    QLabel *dataBusLabel = new QLabel(tr("Data bus"), this);
    m_dataBusLineEdit = new QLineEdit(this);
    m_dataBusLineEdit->setReadOnly(true);
    m_dataBusLineEdit->setFixedWidth(INSTRUCTION_WIDTH);
    m_dataBusLineEdit->setAlignment(Qt::AlignHCenter);
    dataBusLayout->addWidget(dataBusLabel);
    dataBusLayout->addWidget(m_dataBusLineEdit);

    busesGroupLayout->addLayout(addressBusLayout);
    busesGroupLayout->addLayout(dataBusLayout);
    busesGroupBox->setLayout(busesGroupLayout);
    busesGroupBox->setAlignment(Qt::AlignHCenter);
    // =============================


    // ==== NUMBER BASE BUTTONS ====
    m_binaryBaseButton = new QPushButton(tr("Binary"), this);
    m_decimalBaseButton = new QPushButton(tr("Decimal"), this);
    m_hexadecimalBaseButton = new QPushButton(tr("Hexadecimal"), this);
    QHBoxLayout *baseButtonsLayout = new QHBoxLayout;
    baseButtonsLayout->addWidget(m_binaryBaseButton);
    baseButtonsLayout->addWidget(m_decimalBaseButton);
    baseButtonsLayout->addWidget(m_hexadecimalBaseButton);
    // =============================


    QPushButton *closeButton = new QPushButton(tr("Close"), this);


    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(stateGroupBox);
    mainLayout->addWidget(registersGroupBox);
    mainLayout->addWidget(stackGroupBox);
    mainLayout->addWidget(busesGroupBox);
    mainLayout->addLayout(baseButtonsLayout);
    mainLayout->addWidget(closeButton);

    setLayout(mainLayout);

    connect(m_binaryBaseButton, SIGNAL(clicked()), this, SLOT(switchToBinaryBase()));
    connect(m_decimalBaseButton, SIGNAL(clicked()), this, SLOT(switchToDecimalBase()));
    connect(m_hexadecimalBaseButton, SIGNAL(clicked()), this, SLOT(switchToHexadecimalBase()));
    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));

    updateStatus();
}

void CpuStateViewer::updateStatus()
{
    if (m_savedState.lastState)
    {
        m_lastStateLabel->show();
    }
    else
    {
        m_lastStateLabel->hide();
    }

    if (m_savedState.state == Cpu::CpuState::INSTRUCTION_EXEC)
    {
        if (m_savedState.flags[(int)Cpu::Flags::HALT])
        {
            m_stateLineEdit->setText("HALTED");
            m_stateLineEdit->setStyleSheet("color: red;");
        }
        else
        {
            m_stateLineEdit->setText("EXECUTING INSTRUCTION");
            m_stateLineEdit->setStyleSheet("color: green;");
        }
    }
    else if (m_savedState.state == Cpu::CpuState::INTERRUPT_MANAGEMENT)
    {
        m_stateLineEdit->setText("ANALYZING INTERRUPT");
        m_stateLineEdit->setStyleSheet("color: orange;");
    }
    else
    {
        m_stateLineEdit->setText("ERROR (INVALID)");
        m_stateLineEdit->setStyleSheet("color: red;");
    }

    if (m_savedState.interruptReady)
    {
        m_interruptReadyLineEdit->setText("INTERRUPT READY");
        m_interruptReadyLineEdit->setStyleSheet("color: green;");
    }
    else
    {
        m_interruptReadyLineEdit->setText("INTERRUPTS QUEUED");
        m_interruptReadyLineEdit->setStyleSheet("color: red;");
    }

    switch (m_savedState.addrMode)
    {
        case Cpu::AddressingMode::NONE:
            m_addressingModeLineEdit->setText("NONE");
            break;

        case Cpu::AddressingMode::REG:
            m_addressingModeLineEdit->setText("REG");
            break;

        case Cpu::AddressingMode::REG_IMM8:
            m_addressingModeLineEdit->setText("REG IMM8");
            break;

        case Cpu::AddressingMode::REG_RAM:
            m_addressingModeLineEdit->setText("REG RAM");
            break;

        case Cpu::AddressingMode::RAMREG_IMMREG:
            m_addressingModeLineEdit->setText("RAMREG IMMREG");
            break;

        case Cpu::AddressingMode::REG16:
            m_addressingModeLineEdit->setText("REG16");
            break;

        case Cpu::AddressingMode::IMM16:
            m_addressingModeLineEdit->setText("IMM16");
            break;

        case Cpu::AddressingMode::IMM8:
            m_addressingModeLineEdit->setText("IMM8");
            break;

        default:
            m_addressingModeLineEdit->setText("INVALID");
    }

    for (unsigned int i(0); i < Cpu::FLAGS_NB; i++)
    {
        m_flagsTableItem[i]->setText((m_savedState.flags[i]) ? "1" : "0");
    }

    changeValueBase();
}

void CpuStateViewer::changeValueBase()
{
    setFixedWidth((m_base == Base::BINARY) ? CPU_STATE_VIEWER_WIDTH_BINARY : CPU_STATE_VIEWER_WIDTH);

    // Widget width
    m_decodedInstructionTable->setFixedSize((m_base == Base::BINARY) ? DECODED_INSTRUCTION_TABLE_WIDTH_BINARY : DECODED_INSTRUCTION_TABLE_WIDTH, TABLE_HEIGHT);
    m_decodedInstructionTable->setColumnWidth(5, (m_base == Base::BINARY) ? BYTE_ITEM_WIDTH : REG_ITEM_WIDTH);
    m_decodedInstructionTable->setColumnWidth(6, (m_base == Base::BINARY) ? BYTE_ITEM_WIDTH : REG_ITEM_WIDTH);
    m_decodedInstructionTable->setColumnWidth(7, (m_base == Base::BINARY) ? WORD_ITEM_WIDTH_BINARY : WORD_ITEM_WIDTH);

    // Values
    m_programCounterLineEdit->setText(word2QString(m_savedState.programCounter, m_base));
    m_instructionRegisterLineEdit->setText(dWord2QString(m_savedState.instructionRegister, m_base));

    m_opcodeTableItem->setText(byte2QString((Byte)m_savedState.opcode, m_base, (m_base == Base::BINARY ? 6 : 0)));
    m_addrModeTableItem->setText(byte2QString((Byte)m_savedState.addrMode, m_base, (m_base == Base::BINARY ? 4 : 0)));
    m_r1TableItem->setText(byte2QString((Byte)m_savedState.r1, m_base, (m_base == Base::BINARY ? 3 : 0)));
    m_r2TableItem->setText(byte2QString((Byte)m_savedState.r2, m_base, (m_base == Base::BINARY ? 3 : 0)));
    m_r3TableItem->setText(byte2QString((Byte)m_savedState.r3, m_base, (m_base == Base::BINARY ? 3 : 0)));
    m_v1TableItem->setText(byte2QString(m_savedState.v1, m_base));
    m_v2TableItem->setText(byte2QString(m_savedState.v2, m_base));
    m_vXTableItem->setText(word2QString(m_savedState.vX, m_base));

    for (unsigned int i(0); i < Cpu::REGISTERS_NB; i++)
    {
        m_registersLineEdits[i]->setText(byte2QString(m_savedState.registers[i], m_base));
    }

    m_stackPointerLineEdit->setText(byte2QString(m_savedState.stackPointer, m_base));

    m_addressBusLineEdit->setText(word2QString(m_savedState.addressBus, m_base));
    m_dataBusLineEdit->setText(byte2QString(m_savedState.dataBus, m_base));
}
