#include "cpuStateViewer.h"

CpuStateViewer* CpuStateViewer::m_singleton = nullptr;

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
    if (m_singleton != nullptr)
        m_singleton->updateStatus(status);
}

QString CpuStateViewer::byte2QString(Byte value)
{
    QString result("0x");

    if (value < 0x10)
        result += "0";

    result += QString::number(value, 16).toUpper();

    return result;
}

QString CpuStateViewer::word2QString(Word value)
{
    QString result("0x");

    if (value < 0x1000)
        result += "0";
    if (value < 0x100)
        result += "0";
    if (value < 0x10)
        result += "0";

    result += QString::number(value, 16).toUpper();

    return result;
}

QString CpuStateViewer::dWord2QString(Dword value)
{
    QString result("0x");

    if (value < 0x10000000)
        result += "0";
    if (value < 0x1000000)
        result += "0";
    if (value < 0x100000)
        result += "0";
    if (value < 0x10000)
        result += "0";
    if (value < 0x1000)
        result += "0";
    if (value < 0x100)
        result += "0";
    if (value < 0x10)
        result += "0";

    result += QString::number(value, 16).toUpper();

    return result;
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

// PRIVATE
CpuStateViewer::CpuStateViewer(QWidget *parent) : QDialog(parent)
{
    setWindowTitle(tr("CPU State"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    setFixedWidth(CPU_STATE_VIEWER_WIDTH);

    // ==== STATE GROUP BOX (VLayout) ====
    QGroupBox *stateGroupBox = new QGroupBox(tr("Status"), this);
    QVBoxLayout *stateGroupLayout = new QVBoxLayout;

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

    // Decoded instruction table

    stateGroupLayout->addWidget(m_stateLineEdit);
    stateGroupLayout->setAlignment(m_stateLineEdit, Qt::AlignHCenter);
    stateGroupLayout->addWidget(m_interruptReadyLineEdit);
    stateGroupLayout->setAlignment(m_interruptReadyLineEdit, Qt::AlignHCenter);
    stateGroupLayout->addLayout(programCounterLayout);
    stateGroupLayout->addLayout(instructionRegisterLayout);
    stateGroupLayout->addLayout(adressingModeLayout);
    stateGroupBox->setLayout(stateGroupLayout);
    stateGroupBox->setAlignment(Qt::AlignHCenter);
    // ===================================

    // ==== REGISTERS GROUP BOX (VLayout) ====
    QGroupBox *registersGroupBox = new QGroupBox(tr("Registers"), this);
    QVBoxLayout *registersGroupLayout = new QVBoxLayout;

    QLabel *flagsLabel = new QLabel(tr("Flags"), this);
    // Flags table (read-only)

    QHBoxLayout *mainRegistersLayout = new QHBoxLayout;
        QVBoxLayout *registerADLayout = new QVBoxLayout;

            QHBoxLayout *registerALayout = new QHBoxLayout;
            QLabel *registerALabel = new QLabel(tr("A"), this);
            m_registerALineEdit = new QLineEdit(this);
            m_registerALineEdit->setReadOnly(true);
            m_registerALineEdit->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registerALineEdit->setAlignment(Qt::AlignHCenter);
            registerALayout->addWidget(m_registerALineEdit);
            registerALayout->addWidget(registerALabel);
            registerALayout->setAlignment(m_registerALineEdit, Qt::AlignHCenter);
            registerALayout->setAlignment(registerALabel, Qt::AlignHCenter);

            QHBoxLayout *registerBLayout = new QHBoxLayout;
            QLabel *registerBLabel = new QLabel(tr("B"), this);
            m_registerBLineEdit = new QLineEdit(this);
            m_registerBLineEdit->setReadOnly(true);
            m_registerBLineEdit->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registerBLineEdit->setAlignment(Qt::AlignHCenter);
            registerBLayout->addWidget(m_registerBLineEdit);
            registerBLayout->addWidget(registerBLabel);
            registerBLayout->setAlignment(m_registerBLineEdit, Qt::AlignHCenter);
            registerBLayout->setAlignment(registerBLabel, Qt::AlignHCenter);

            QHBoxLayout *registerCLayout = new QHBoxLayout;
            QLabel *registerCLabel = new QLabel(tr("C"), this);
            m_registerCLineEdit = new QLineEdit(this);
            m_registerCLineEdit->setReadOnly(true);
            m_registerCLineEdit->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registerCLineEdit->setAlignment(Qt::AlignHCenter);
            registerCLayout->addWidget(m_registerCLineEdit);
            registerCLayout->addWidget(registerCLabel);
            registerCLayout->setAlignment(m_registerCLineEdit, Qt::AlignHCenter);
            registerCLayout->setAlignment(registerCLabel, Qt::AlignHCenter);

            QHBoxLayout *registerDLayout = new QHBoxLayout;
            QLabel *registerDLabel = new QLabel(tr("D"), this);
            m_registerDLineEdit = new QLineEdit(this);
            m_registerDLineEdit->setReadOnly(true);
            m_registerDLineEdit->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registerDLineEdit->setAlignment(Qt::AlignHCenter);
            registerDLayout->addWidget(m_registerDLineEdit);
            registerDLayout->addWidget(registerDLabel);
            registerDLayout->setAlignment(m_registerDLineEdit, Qt::AlignHCenter);
            registerDLayout->setAlignment(registerDLabel, Qt::AlignHCenter);

        registerADLayout->addLayout(registerALayout);
        registerADLayout->addLayout(registerBLayout);
        registerADLayout->addLayout(registerCLayout);
        registerADLayout->addLayout(registerDLayout);

        QVBoxLayout *registerIXLayout = new QVBoxLayout;

            QHBoxLayout *registerILayout = new QHBoxLayout;
            QLabel *registerILabel = new QLabel(tr("I"), this);
            m_registerILineEdit = new QLineEdit(this);
            m_registerILineEdit->setReadOnly(true);
            m_registerILineEdit->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registerILineEdit->setAlignment(Qt::AlignHCenter);
            registerILayout->addWidget(registerILabel);
            registerILayout->addWidget(m_registerILineEdit);
            registerILayout->setAlignment(registerILabel, Qt::AlignHCenter);
            registerILayout->setAlignment(m_registerILineEdit, Qt::AlignHCenter);

            QHBoxLayout *registerJLayout = new QHBoxLayout;
            QLabel *registerJLabel = new QLabel(tr("J"), this);
            m_registerJLineEdit = new QLineEdit(this);
            m_registerJLineEdit->setReadOnly(true);
            m_registerJLineEdit->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registerJLineEdit->setAlignment(Qt::AlignHCenter);
            registerJLayout->addWidget(registerJLabel);
            registerJLayout->addWidget(m_registerJLineEdit);
            registerJLayout->setAlignment(registerJLabel, Qt::AlignHCenter);
            registerJLayout->setAlignment(m_registerJLineEdit, Qt::AlignHCenter);

            QHBoxLayout *registerXLayout = new QHBoxLayout;
            QLabel *registerXLabel = new QLabel(tr("X"), this);
            m_registerXLineEdit = new QLineEdit(this);
            m_registerXLineEdit->setReadOnly(true);
            m_registerXLineEdit->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registerXLineEdit->setAlignment(Qt::AlignHCenter);
            registerXLayout->addWidget(registerXLabel);
            registerXLayout->addWidget(m_registerXLineEdit);
            registerXLayout->setAlignment(registerXLabel, Qt::AlignHCenter);
            registerXLayout->setAlignment(m_registerXLineEdit, Qt::AlignHCenter);

            QHBoxLayout *registerYLayout = new QHBoxLayout;
            QLabel *registerYLabel = new QLabel(tr("Y"), this);
            m_registerYLineEdit = new QLineEdit(this);
            m_registerYLineEdit->setReadOnly(true);
            m_registerYLineEdit->setFixedWidth(REGISTER_LINE_EDIT_WIDTH);
            m_registerYLineEdit->setAlignment(Qt::AlignHCenter);
            registerYLayout->addWidget(registerYLabel);
            registerYLayout->addWidget(m_registerYLineEdit);
            registerYLayout->setAlignment(registerYLabel, Qt::AlignHCenter);
            registerYLayout->setAlignment(m_registerYLineEdit, Qt::AlignHCenter);

        registerIXLayout->addLayout(registerILayout);
        registerIXLayout->addLayout(registerJLayout);
        registerIXLayout->addLayout(registerXLayout);
        registerIXLayout->addLayout(registerYLayout);

    mainRegistersLayout->addLayout(registerADLayout);
    mainRegistersLayout->addLayout(registerIXLayout);

    registersGroupLayout->addWidget(flagsLabel);
    registersGroupLayout->setAlignment(flagsLabel, Qt::AlignHCenter);
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


    QPushButton *closeButton = new QPushButton(tr("Close"), this);


    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(stateGroupBox);
    mainLayout->addWidget(registersGroupBox);
    mainLayout->addWidget(stackGroupBox);
    mainLayout->addWidget(busesGroupBox);
    mainLayout->addWidget(closeButton);

    setLayout(mainLayout);

    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));
}

void CpuStateViewer::updateStatus(const CpuStatus status)
{
    m_stateLineEdit->setText((status.state == Cpu::CpuState::INSTRUCTION_EXEC) ? "EXECUTING INSTRUCTION" : "ANALIZING INTERRUPT");
    m_interruptReadyLineEdit->setText(status.interruptReady ? "Yes" : "No");

    m_programCounterLineEdit->setText(word2QString(status.programCounter));
    m_instructionRegisterLineEdit->setText(dWord2QString(status.instructionRegister));

    switch (status.addrMode)
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

    m_registerALineEdit->setText(byte2QString(status.regA));
    m_registerBLineEdit->setText(byte2QString(status.regB));
    m_registerCLineEdit->setText(byte2QString(status.regC));
    m_registerDLineEdit->setText(byte2QString(status.regD));
    m_registerILineEdit->setText(byte2QString(status.regI));
    m_registerJLineEdit->setText(byte2QString(status.regJ));
    m_registerXLineEdit->setText(byte2QString(status.regX));
    m_registerYLineEdit->setText(byte2QString(status.regY));

    m_stackPointerLineEdit->setText(word2QString(status.stackPointer));

    m_addressBusLineEdit->setText(word2QString(status.addressBus));
    m_dataBusLineEdit->setText(byte2QString(status.dataBus));
}
