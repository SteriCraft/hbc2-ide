#include "mainWindow.h"

HbcEmulator* HbcEmulator::m_singleton = nullptr;

// PUBLIC
HbcEmulator* HbcEmulator::getInstance(MainWindow *mainWin, Console *consoleOutput)
{
    if (m_singleton == nullptr)
        m_singleton = new HbcEmulator(mainWin, consoleOutput);

    return m_singleton;
}

HbcEmulator::~HbcEmulator()
{
    m_status.mutex.lock();
    m_status.command = Emulator::Command::CLOSE;
    m_status.mutex.unlock();

    wait();

    m_singleton = nullptr;
}

bool HbcEmulator::runCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState == Emulator::State::READY)
    {
        m_status.mutex.lock();
        if (m_status.startPaused)
        {
            m_status.command = Emulator::Command::PAUSE;
        }
        else
        {
            m_status.command = Emulator::Command::RUN;
        }
        m_status.mutex.unlock();

        success = true;
    }
    else if (currentState == Emulator::State::PAUSED)
    {
        m_status.mutex.lock();
        m_status.command = Emulator::Command::RUN;
        m_status.mutex.unlock();

        success = true;
    }

    return success;
}

bool HbcEmulator::stepCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState == Emulator::State::READY || currentState == Emulator::State::PAUSED)
    {
        m_status.mutex.lock();
        m_status.command = Emulator::Command::STEP;
        m_status.mutex.unlock();

        success = true;
    }

    return success;
}

bool HbcEmulator::pauseCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState == Emulator::State::RUNNING)
    {
        qDebug() << "[MAINWIN]: Pausing emulator";

        m_status.mutex.lock();
        m_status.command = Emulator::Command::PAUSE;
        m_status.mutex.unlock();

        success = true;
    }

    return success;
}

bool HbcEmulator::stopCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState != Emulator::State::NOT_INITIALIZED)
    {
        qDebug() << "[MAINWIN]: Stopping emulator";

        m_status.mutex.lock();
        m_status.command = Emulator::Command::STOP;
        m_status.mutex.unlock();

        success = true;
    }

    return success;
}

bool HbcEmulator::loadProject(QString romBinaryFilePath, QString projectName)
{
    bool success = false;

    if (getState() == Emulator::State::NOT_INITIALIZED || getState() == Emulator::State::READY)
    {
        m_status.projectName = projectName.toStdString();
        m_computer.initialRamData.clear();

        for (unsigned int i(0); i < m_computer.peripherals.size(); i++)
        {
            delete m_computer.peripherals[i];
        }
        m_computer.peripherals.clear();

        if (m_status.useMonitor)
        {
            m_computer.peripherals.push_back(new HbcMonitor(&m_computer.motherboard.m_iod, m_consoleOutput));
            m_computer.monitor = dynamic_cast<HbcMonitor*>(m_computer.peripherals.back());
        }

        if (m_status.useRTC)
        {
            m_computer.peripherals.push_back(new RealTimeClock::HbcRealTimeClock(&m_computer.motherboard.m_iod, m_consoleOutput));
            m_computer.rtc = dynamic_cast<RealTimeClock::HbcRealTimeClock*>(m_computer.peripherals.back());
        }

        if (m_status.useKeyboard)
        {
            m_computer.peripherals.push_back(new Keyboard::HbcKeyboard(&m_computer.motherboard.m_iod, m_consoleOutput));
            m_computer.keyboard = dynamic_cast<Keyboard::HbcKeyboard*>(m_computer.peripherals.back());
        }

        // EEPROM always loaded because this function is called only if the EEPROM is plugged in by the IDE
        m_computer.peripherals.push_back(new Eeprom::HbcEeprom(romBinaryFilePath, &m_computer.motherboard.m_ram, &m_computer.motherboard.m_iod, m_consoleOutput));
        m_computer.eeprom = dynamic_cast<Eeprom::HbcEeprom*>(m_computer.peripherals.back());

        initComputer();

        m_status.state = Emulator::State::READY;
        success = true;
    }
    else
    {
        m_consoleOutput->log("Unable to set initial binary data because the emulator is running");
        m_consoleOutput->returnLine();
    }

    return success;
}

bool HbcEmulator::loadProject(QByteArray initialRamData, QString projectName)
{
    bool success = false;

    if (getState() == Emulator::State::NOT_INITIALIZED || getState() == Emulator::State::READY)
    {
        if (initialRamData.size() == Ram::MEMORY_SIZE)
        {
            m_status.projectName = projectName.toStdString();
            m_computer.initialRamData = initialRamData;

            for (unsigned int i(0); i < m_computer.peripherals.size(); i++)
            {
                delete m_computer.peripherals[i];
            }
            m_computer.peripherals.clear();

            if (m_status.useMonitor)
            {
                m_computer.peripherals.push_back(new HbcMonitor(&m_computer.motherboard.m_iod, m_consoleOutput));
                m_computer.monitor = dynamic_cast<HbcMonitor*>(m_computer.peripherals.back());
            }

            if (m_status.useRTC)
            {
                m_computer.peripherals.push_back(new RealTimeClock::HbcRealTimeClock(&m_computer.motherboard.m_iod, m_consoleOutput));
                m_computer.rtc = dynamic_cast<RealTimeClock::HbcRealTimeClock*>(m_computer.peripherals.back());
            }

            if (m_status.useKeyboard)
            {
                m_computer.peripherals.push_back(new Keyboard::HbcKeyboard(&m_computer.motherboard.m_iod, m_consoleOutput));
                m_computer.keyboard = dynamic_cast<Keyboard::HbcKeyboard*>(m_computer.peripherals.back());
            }

            // EEPROM never loaded because this function is only called if the EEPROM is not plugged in by the IDE

            initComputer();

            m_status.state = Emulator::State::READY;
            success = true;
        }
        else
        {
            m_consoleOutput->log("Invalid initial binary data size (must be 65'536 bytes large)");
            m_consoleOutput->returnLine();
        }
    }
    else
    {
        m_consoleOutput->log("Unable to set initial binary data because the emulator is running");
        m_consoleOutput->returnLine();
    }

    return success;
}

const QByteArray HbcEmulator::getCurrentRamBinaryData()
{
    QByteArray exportContent;

    m_computer.motherboard.m_ram.mutex.lock();

    for (unsigned int i(0); i < Ram::MEMORY_SIZE; i++)
    {
        exportContent.push_back(m_computer.motherboard.m_ram.memory[i]);
    }

    m_computer.motherboard.m_ram.mutex.unlock();

    return exportContent;
}

const QByteArray HbcEmulator::getCurrentEepromBinaryData()
{
    QByteArray exportContent;

    if (m_computer.eeprom != nullptr)
    {
        exportContent = m_computer.eeprom->getMemoryContent();
    }

    return exportContent;
}

const CpuStatus HbcEmulator::getCurrentCpuStatus()
{
    return m_computer.cpuState;
}

const Word HbcEmulator::getCurrentProgramCounter()
{
    return m_computer.cpuState.programCounter;
}

void HbcEmulator::useMonitor(bool enable)
{
    m_status.useMonitor = enable;
}

void HbcEmulator::useRTC(bool enable)
{
    m_status.useRTC = enable;
}

void HbcEmulator::useKeyboard(bool enable)
{
    m_status.useKeyboard = enable;
}

void HbcEmulator::setStartPaused(bool enable)
{
    m_status.startPaused = enable;
}

Emulator::State HbcEmulator::getState()
{
    Emulator::State currentState;

    m_status.mutex.lock();
    currentState = m_status.state;
    m_status.mutex.unlock();

    return currentState;
}

Emulator::FrequencyTarget HbcEmulator::getFrequencyTarget()
{
    Emulator::FrequencyTarget frequencyTarget;

    m_status.mutex.lock();
    frequencyTarget = m_status.frequencyTarget;
    m_status.mutex.unlock();

    return frequencyTarget;
}

HbcMonitor* HbcEmulator::getHbcMonitor()
{
    return m_computer.monitor;
}

RealTimeClock::HbcRealTimeClock* HbcEmulator::getHbcRealTimeClock()
{
    return m_computer.rtc;
}

Keyboard::HbcKeyboard* HbcEmulator::getHbcKeyboard()
{
    return m_computer.keyboard;
}

void HbcEmulator::setFrequencyTarget(Emulator::FrequencyTarget target)
{
    m_status.mutex.lock();
    m_status.frequencyTarget = target;
    m_status.mutex.unlock();
}

void HbcEmulator::setBreakpoints(std::vector<Word> breakpoints)
{
    m_computer.breakpoints = breakpoints;
}

// PRIVATE
HbcEmulator::HbcEmulator(MainWindow *mainWin, Console *consoleOutput)
{
    m_status.state = Emulator::State::NOT_INITIALIZED;
    m_status.command = Emulator::Command::NONE;
    m_status.frequencyTarget = Emulator::FrequencyTarget::MHZ_2; // Default
    m_status.useMonitor = true;
    m_status.useRTC = true;
    m_status.useKeyboard = true;

    m_consoleOutput = consoleOutput;
    m_mainWindow = mainWin;

    connect(this, SIGNAL(statusChanged(Emulator::State)), m_mainWindow, SLOT(onEmulatorStatusChanged(Emulator::State)), Qt::ConnectionType::BlockingQueuedConnection);
    connect(this, SIGNAL(tickCountSent(int)), m_mainWindow, SLOT(onTickCountReceived(int)), Qt::ConnectionType::BlockingQueuedConnection);
    connect(this, SIGNAL(stepped()), m_mainWindow, SLOT(onEmulatorStepped()), Qt::ConnectionType::BlockingQueuedConnection);

    start(); // Threads always idling
}

void HbcEmulator::run()
{
    bool stop(false);
    Emulator::State currentState(getState());
    Emulator::FrequencyTarget frequencyTarget(getFrequencyTarget());

    QElapsedTimer frequencyTimer, commandsTimer, frequencyTargetTimer;

    int ticks(0), targetTicks(0);

    commandsTimer.start();
    frequencyTargetTimer.start();
    while (!stop)
    {
        // --- EXECUTION ---
        if (currentState == Emulator::State::RUNNING)
        {
            // --- CPU SPEED CONTROL ---
            if (frequencyTarget == Emulator::FrequencyTarget::FASTEST)
            {
                tickComputer();
                ticks++;
            }
            else
            {
                if (frequencyTargetTimer.elapsed() >= 5)
                {
                    targetTicks = 0;
                    frequencyTargetTimer.restart();
                }

                if (targetTicks <= ((int)frequencyTarget / 200))
                {
                    tickComputer();

                    ticks++;
                    targetTicks++;
                }
            }

            // --- CPU SPEED DISPLAY ---
            if (frequencyTimer.elapsed() >= 1000) // in ms
            {
                emit tickCountSent(ticks);
                ticks = 0;

                frequencyTimer.restart();
            }

            // --- BREAKPOINT CHECK ---
            for (unsigned int i(0); i < m_computer.breakpoints.size(); i++)
            {
                if (m_computer.motherboard.m_cpu.m_programCounter == m_computer.breakpoints[i])
                {
                    m_status.mutex.lock();
                    m_status.state = Emulator::State::PAUSED;
                    currentState = m_status.state;
                    storeCpuStatus();
                    m_status.mutex.unlock();

                    m_consoleOutput->log(tr("Breakpoint reached at address ") + word2QString(m_computer.breakpoints[i]));

                    emit statusChanged(currentState);
                    break;
                }
            }
        }

        // --- COMMANDS CHECKS ---
        if (commandsTimer.elapsed() >= 100) // in ms
        {
            Emulator::Command executedCommand; // To emit signals later (to avoid threads blocking each other)
            m_status.mutex.lock();

            frequencyTarget = m_status.frequencyTarget;

            executedCommand = m_status.command;
            if (m_status.command == Emulator::Command::RUN)
            {
                m_status.state = Emulator::State::RUNNING;
                m_status.command = Emulator::Command::NONE;

                frequencyTimer.restart();
                frequencyTargetTimer.restart();

                m_consoleOutput->log("Emulator running");
            }
            else if (m_status.command == Emulator::Command::STEP)
            {
                m_status.command = Emulator::Command::NONE;

                tickComputer(true);
                storeCpuStatus();
            }
            else if (m_status.command == Emulator::Command::PAUSE)
            {
                m_status.state = Emulator::State::PAUSED;
                m_status.command = Emulator::Command::NONE;

                storeCpuStatus();

                m_consoleOutput->log("Emulator paused");
            }
            else if (m_status.command == Emulator::Command::STOP)
            {
                qDebug() << "EMULATOR STOP COMMAND EXECUTED"; // TODO: for debug of persisting thread on closure
                m_status.state = Emulator::State::READY;
                m_status.command = Emulator::Command::NONE;

                storeCpuStatus(true);

                initComputer();

                m_consoleOutput->log("Emulator stopped");
            }
            else if (m_status.command == Emulator::Command::CLOSE)
            {
                qDebug() << "EMULATOR CLOSE COMMAND EXECUTED"; // TODO: for debug of persisting thread on closure
                m_status.state = Emulator::State::NOT_INITIALIZED;
                m_status.command = Emulator::Command::NONE;

                stop = true;
            }

            currentState = m_status.state;
            m_status.mutex.unlock();

            if (executedCommand == Emulator::Command::RUN || executedCommand == Emulator::Command::PAUSE || executedCommand == Emulator::Command::STOP)
            {
                emit statusChanged(currentState);
            }
            else if (executedCommand == Emulator::Command::STEP)
            {
                emit stepped();
            }

            commandsTimer.restart();
        }
    }
}

void HbcEmulator::initComputer()
{
    Motherboard::init(m_computer.motherboard, m_computer.initialRamData);

    for (unsigned int i(0); i < m_computer.peripherals.size(); i++)
    {
        m_computer.peripherals[i]->init();
    }
}

void HbcEmulator::tickComputer(bool step)
{
    Motherboard::tick(m_computer.motherboard);

    for (unsigned int i(0); i < m_computer.peripherals.size(); i++)
    {
        m_computer.peripherals[i]->tick(step);
    }
}

void HbcEmulator::storeCpuStatus(bool lastState)
{
    m_computer.cpuState.lastState = lastState;

    m_computer.cpuState.state = m_computer.motherboard.m_cpu.m_currentState;
    m_computer.cpuState.interruptReady = (m_computer.motherboard.m_cpu.m_flags[(int)Cpu::Flags::INTERRUPT]) ? true : false;

    m_computer.cpuState.programCounter = m_computer.motherboard.m_cpu.m_lastExecutedInstructionAddress;
    m_computer.cpuState.instructionRegister = m_computer.motherboard.m_cpu.m_instructionRegister;

    // == DECODED INSTRUCTION ==
    m_computer.cpuState.opcode = m_computer.motherboard.m_cpu.m_opcode;
    m_computer.cpuState.addrMode = m_computer.motherboard.m_cpu.m_addressingMode;

    m_computer.cpuState.r1 = m_computer.motherboard.m_cpu.m_register1Index;
    m_computer.cpuState.r2 = m_computer.motherboard.m_cpu.m_register2Index;
    m_computer.cpuState.r3 = m_computer.motherboard.m_cpu.m_register3Index;

    m_computer.cpuState.v1 = m_computer.motherboard.m_cpu.m_v1;
    m_computer.cpuState.v2 = m_computer.motherboard.m_cpu.m_v2;
    m_computer.cpuState.vX = m_computer.motherboard.m_cpu.m_vX;
    // =========================

    for (unsigned int i(0); i < Cpu::FLAGS_NB; i++)
    {
        m_computer.cpuState.flags[i] = m_computer.motherboard.m_cpu.m_flags[i];
    }

    for (unsigned int i(0); i < Cpu::REGISTERS_NB; i++)
    {
        m_computer.cpuState.registers[i] = m_computer.motherboard.m_cpu.m_registers[i];
    }

    m_computer.cpuState.stackPointer = m_computer.motherboard.m_cpu.m_stackPointer;

    m_computer.cpuState.addressBus = m_computer.motherboard.m_addressBus;
    m_computer.cpuState.dataBus = m_computer.motherboard.m_dataBus;
}
