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
    if (isRunning())
    {
        m_status.m_lock.lock();
        m_status.m_command = Emulator::Command::CLOSE_APP;
        m_status.m_lock.unlock();

        wait();

        m_consoleOutput->log("Emulator stopped");
    }
}

void HbcEmulator::run()
{
    bool stop = false;
    Emulator::State currentStatus = Emulator::State::PAUSED;

    QElapsedTimer timer;
    timer.start();

    qDebug() << "[EMULATOR]: Start";

    m_computer.m_lock.lock();

    while (!stop)
    {
        if (currentStatus == Emulator::State::RUNNING)
        {
            Motherboard::tick(m_computer.m_motherboard);
        }

        if (timer.elapsed() >= 100) // in ms
        {
            m_status.m_lock.lock();

            switch (m_status.m_command)
            {
                case Emulator::Command::RUN:
                    m_status.m_state = Emulator::State::RUNNING;
                    m_status.m_command = Emulator::Command::NONE;
                    currentStatus = m_status.m_state;
                    emit statusChanged(m_status.m_state);

                    qDebug() << "[EMULATOR]: Full speed mode";
                    break;

                case Emulator::Command::STEP:
                    Motherboard::tick(m_computer.m_motherboard);

                    m_status.m_state = Emulator::State::PAUSED;
                    m_status.m_command = Emulator::Command::NONE;
                    currentStatus = m_status.m_state;
                    emit statusChanged(m_status.m_state);

                    qDebug() << "[EMULATOR]: Step forward";
                    break;

                case Emulator::Command::PAUSE:
                    m_status.m_state = Emulator::State::PAUSED;
                    m_status.m_command = Emulator::Command::NONE;
                    currentStatus = m_status.m_state;
                    emit statusChanged(m_status.m_state);

                    qDebug() << "[EMULATOR]: Pause";
                    break;

                case Emulator::Command::CLOSE:
                    m_status.m_state = Emulator::State::NOT_INITIALIZED;
                    m_status.m_command = Emulator::Command::NONE;
                    emit statusChanged(m_status.m_state);

                    stop = true;
                    qDebug() << "[EMULATOR]: Close";
                    break;

                case Emulator::Command::CLOSE_APP:
                    m_status.m_state = Emulator::State::NOT_INITIALIZED;
                    m_status.m_command = Emulator::Command::NONE;

                    stop = true;
                    qDebug() << "[EMULATOR]: Close";
                    break;

                case Emulator::Command::NONE:
                    break;
            }

            m_status.m_lock.unlock();
            timer.restart();
        }
    }

    m_computer.m_lock.unlock();
}

bool HbcEmulator::runCmd()
{
    bool success = false;
    Emulator::State currentState = getState();

    if (currentState == Emulator::State::PAUSED)
    {
        m_status.m_lock.lock();
        m_status.m_command = Emulator::Command::RUN;
        m_status.m_lock.unlock();

        success = true;

        m_consoleOutput->log("Emulator runs at full speed");
    }
    else if (currentState == Emulator::State::RUNNING)
    {
        m_consoleOutput->log("Emulation is already running");
    }
    else
    {
        m_consoleOutput->log("Emulation is not initialized");
    }

    return success;
}

bool HbcEmulator::stepCmd()
{
    bool success = false;
    Emulator::State currentState = getState();

    if (currentState == Emulator::State::PAUSED)
    {
        m_status.m_lock.lock();
        m_status.m_command = Emulator::Command::STEP;
        m_status.m_lock.unlock();

        success = true;

        m_consoleOutput->log("Emulator steps forward");
    }
    else if (currentState == Emulator::State::RUNNING)
    {
        m_consoleOutput->log("Emulator is already running");
    }
    else
    {
        m_consoleOutput->log("Emulator is not initialized");
    }

    return success;
}

bool HbcEmulator::pauseCmd()
{
    bool success = false;
    Emulator::State currentState = getState();

    if (currentState == Emulator::State::RUNNING)
    {
        m_status.m_lock.lock();
        m_status.m_command = Emulator::Command::PAUSE;
        m_status.m_lock.unlock();

        success = true;

        m_consoleOutput->log("Emulator paused");
    }
    else if (currentState == Emulator::State::PAUSED)
    {
        m_consoleOutput->log("Emulator is not initialized");
    }
    else
    {
        m_consoleOutput->log("Emulator is already paused");
    }

    return success;
}

bool HbcEmulator::stopCmd()
{
    bool success = false;

    if (getState() != Emulator::State::NOT_INITIALIZED)
    {
        m_status.m_lock.lock();
        m_status.m_command = Emulator::Command::CLOSE;
        m_status.m_lock.unlock();

        success = true;

        m_consoleOutput->log("Emulator stopped");
    }

    return success;
}

bool HbcEmulator::loadProject(QByteArray data, std::string projectName)
{
    bool success = false;

    if (getState() == Emulator::State::NOT_INITIALIZED)
    {
        if (data.size() == MEMORY_SIZE)
        {
            m_status.m_projectName = projectName;
            m_status.m_state = Emulator::State::PAUSED;

            Motherboard::init(m_computer.m_motherboard, data);
            start();

            success = true;
        }
        else
        {
            m_consoleOutput->log("Invalid initial binary data size (must be 65'536 bytes large)");
        }
    }
    else
    {
        m_consoleOutput->log("Unable to set initial binary data because the emulator is running");
    }

    return success;
}

bool HbcEmulator::loadProject(QByteArray data, QString projectName)
{
    return loadProject(data, projectName.toStdString());
}

Emulator::State HbcEmulator::getState()
{
    Emulator::State emulatorState;

    m_status.m_lock.lock();
    emulatorState = m_status.m_state;
    m_status.m_lock.unlock();

    return emulatorState;
}

// PRIVATE
HbcEmulator::HbcEmulator(MainWindow *mainWin, Console* consoleOutput)
{
    // Emulator settings
    m_status.m_state = Emulator::State::NOT_INITIALIZED;
    m_status.m_projectName = "";

    // Qt settings
    m_consoleOutput = consoleOutput;
    m_mainWindow = mainWin;

    connect(this, SIGNAL(statusChanged(Emulator::State)), m_mainWindow, SLOT(onEmulatorStatusChanged(Emulator::State)), Qt::ConnectionType::BlockingQueuedConnection);
}
