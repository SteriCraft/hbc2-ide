#include "mainwindow.h"

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

bool HbcEmulator::loadProject(QByteArray initialRamData, std::string projectName)
{
    bool success = false;

    if (getState() == Emulator::State::NOT_INITIALIZED || getState() == Emulator::State::READY)
    {
        if (initialRamData.size() == Ram::MEMORY_SIZE)
        {
            m_status.projectName = projectName;
            m_computer.initialRamData = initialRamData;

            m_computer.peripherals.clear();

            if (m_status.useMonitor)
            {
                m_computer.peripherals.push_back(new HbcMonitor(&m_computer.motherboard.m_iod, m_consoleOutput));
                m_computer.monitor = dynamic_cast<HbcMonitor*>(m_computer.peripherals.back());
            }

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

bool HbcEmulator::loadProject(QByteArray initialRamData, QString projectName)
{
    return loadProject(initialRamData, projectName.toStdString());
}

const QByteArray HbcEmulator::getCurrentBinaryData()
{
    QByteArray exportContent;

    m_computer.motherboard.m_ram.m_lock.lock();

    for (unsigned int i(0); i < Ram::MEMORY_SIZE; i++)
    {
        exportContent.push_back(m_computer.motherboard.m_ram.m_memory[i]);
    }

    m_computer.motherboard.m_ram.m_lock.unlock();

    return exportContent;
}

void HbcEmulator::useMonitor(bool enable)
{
    m_status.useMonitor = enable;
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


void HbcEmulator::setFrequencyTarget(Emulator::FrequencyTarget target)
{
    m_status.mutex.lock();
    m_status.frequencyTarget = target;
    m_status.mutex.unlock();
}

// PRIVATE
HbcEmulator::HbcEmulator(MainWindow *mainWin, Console *consoleOutput)
{
    m_status.state = Emulator::State::NOT_INITIALIZED;
    m_status.command = Emulator::Command::NONE;
    m_status.frequencyTarget = Emulator::FrequencyTarget::MHZ_2; // Default
    m_status.useMonitor = true;

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
        }

        // --- COMMANDS CHECKS ---
        if (commandsTimer.elapsed() >= 100) // in ms
        {
            m_status.mutex.lock();

            frequencyTarget = m_status.frequencyTarget;

            if (m_status.command == Emulator::Command::RUN)
            {
                m_status.state = Emulator::State::RUNNING;
                m_status.command = Emulator::Command::NONE;
                emit statusChanged(m_status.state);

                frequencyTimer.restart();
                frequencyTargetTimer.restart();

                m_consoleOutput->log("Emulator running");
            }
            else if (m_status.command == Emulator::Command::STEP)
            {
                m_status.command = Emulator::Command::NONE;

                tickComputer();
                emit stepped();

                m_consoleOutput->log("Emulator steps");
            }
            else if (m_status.command == Emulator::Command::PAUSE)
            {
                m_status.state = Emulator::State::PAUSED;
                m_status.command = Emulator::Command::NONE;
                emit statusChanged(m_status.state);

                m_consoleOutput->log("Emulator paused");
            }
            else if (m_status.command == Emulator::Command::STOP)
            {
                m_status.state = Emulator::State::READY;
                m_status.command = Emulator::Command::NONE;
                emit statusChanged(m_status.state);

                initComputer();

                m_consoleOutput->log("Emulator stopped");
            }
            else if (m_status.command == Emulator::Command::CLOSE)
            {
                m_status.state = Emulator::State::NOT_INITIALIZED;
                m_status.command = Emulator::Command::NONE;

                stop = true;
            }

            currentState = m_status.state;
            m_status.mutex.unlock();

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

void HbcEmulator::tickComputer()
{
    Motherboard::tick(m_computer.motherboard);

    for (unsigned int i(0); i < m_computer.peripherals.size(); i++)
    {
        m_computer.peripherals[i]->tick();
    }
}
