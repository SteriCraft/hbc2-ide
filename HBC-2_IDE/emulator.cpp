#include "mainwindow.h"

HbcEmulator::HbcEmulator(MainWindow *mainWin, Console *consoleOutput)
{
    m_status.state = Emulator::State::NOT_INITIALIZED;
    m_status.command = Emulator::Command::NONE;

    m_consoleOutput = consoleOutput;
    m_mainWindow = mainWin;

    connect(this, SIGNAL(statusChanged(Emulator::State)), m_mainWindow, SLOT(onEmulatorStatusChanged(Emulator::State)), Qt::ConnectionType::BlockingQueuedConnection);

    start(); // Emulator always idling
}

HbcEmulator::~HbcEmulator()
{
    m_status.mutex.lock();
    m_status.command = Emulator::Command::CLOSE;
    m_status.mutex.unlock();

    wait();
}

bool HbcEmulator::runCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState == Emulator::State::READY || currentState == Emulator::State::PAUSED)
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

bool HbcEmulator::loadProject(QByteArray data, std::string projectName)
{
    bool success = false;

    if (getState() == Emulator::State::NOT_INITIALIZED)
    {
        if (data.size() == MEMORY_SIZE)
        {
            m_status.projectName = projectName;

            //m_computer.peripherals.clear();
            // TODO : Plug in peripherals

            //if (m_status.plugMonitor)
            //{
            //m_computer.peripherals.push_back(new HbcMonitor(&m_computer.motherboard.m_iod, m_consoleOutput));
            //}

            //Motherboard::init(m_computer.motherboard, data);

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

bool HbcEmulator::loadProject(QByteArray data, QString projectName)
{
    return loadProject(data, projectName.toStdString());
}

Emulator::State HbcEmulator::getState()
{
    Emulator::State currentState;

    m_status.mutex.lock();
    currentState = m_status.state;
    m_status.mutex.unlock();

    return currentState;
}

void HbcEmulator::run()
{
    bool stop(false);
    Emulator::State currentState(getState());
    QElapsedTimer timer;

    int ticks(0);

    timer.start();
    while (!stop)
    {
        if (currentState == Emulator::State::RUNNING)
        {
            ticks++;
        }

        if (timer.elapsed() >= 100) // in ms
        {
            m_status.mutex.lock();

            // emit signal for ticks number
            ticks = 0;

            if (m_status.command == Emulator::Command::RUN)
            {
                m_status.state = Emulator::State::RUNNING;
                m_status.command = Emulator::Command::NONE;
                emit statusChanged(m_status.state);

                m_consoleOutput->log("Emulator running");
            }
            else if (m_status.command == Emulator::Command::STEP)
            {
                m_status.command = Emulator::Command::NONE;

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
                emit statusChanged(m_status.state); // TODO : Buttons didn't change

                // REINIT computer

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

            timer.restart();
        }
    }
}

/*HbcEmulator* HbcEmulator::m_singleton = nullptr;

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

    qDebug() << "[MAINWIN]: Emulator stopped";
}

bool HbcEmulator::loadProject(QByteArray data, std::string projectName)
{
    bool success = false;

    if (getState() == Emulator::State::NOT_INITIALIZED)
    {
        if (data.size() == MEMORY_SIZE)
        {
            m_status.projectName = projectName;

            //m_computer.peripherals.clear();
            // TODO : Plug in peripherals

            //if (m_status.plugMonitor)
            //{
                //m_computer.peripherals.push_back(new HbcMonitor(&m_computer.motherboard.m_iod, m_consoleOutput));
            //}

            //Motherboard::init(m_computer.motherboard, data);

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

bool HbcEmulator::loadProject(QByteArray data, QString projectName)
{
    return loadProject(data, projectName.toStdString());
}

bool HbcEmulator::runCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState == Emulator::State::NOT_INITIALIZED)
    {
        qDebug() << "[MAINWIN]: Emulator is not initialized";
    }
    else if (currentState == Emulator::State::READY)
    {
        start();

        m_status.mutex.lock();
        m_status.command = Emulator::Command::RUN;
        m_status.mutex.unlock();

        qDebug() << "[MAINWIN]: Emulator starts";
        success = true;
    }
    else if (currentState == Emulator::State::RUNNING)
    {
        qDebug() << "[MAINWIN]: Emulator already running";
    }
    else // PAUSED
    {
        m_status.mutex.lock();
        m_status.command = Emulator::Command::RUN;
        m_status.mutex.unlock();

        qDebug() << "[MAINWIN]: Emulator resumed";
        success = true;
    }

    return success;
}

bool HbcEmulator::pauseCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState == Emulator::State::NOT_INITIALIZED)
    {
        qDebug() << "[MAINWIN]: Emulator is not running";
    }
    else if (currentState == Emulator::State::READY)
    {
        qDebug() << "[MAINWIN]: Emulator is not running";
    }
    else if (currentState == Emulator::State::RUNNING)
    {
        m_status.mutex.lock();
        m_status.command = Emulator::Command::PAUSE;
        m_status.mutex.unlock();

        qDebug() << "[MAINWIN]: Emulator paused";
        success = true;
    }
    else // PAUSED
    {
        qDebug() << "[MAINWIN]: Emulator already paused";
    }

    return success;
}

bool HbcEmulator::stepCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState == Emulator::State::NOT_INITIALIZED)
    {
        qDebug() << "[MAINWIN]: Emulator is not running";
    }
    else if (currentState == Emulator::State::READY)
    {
        m_status.mutex.lock();
        m_status.command = Emulator::Command::STEP;
        m_status.mutex.unlock();

        qDebug() << "[MAINWIN]: Emulator steps";
        success = true;
    }
    else if (currentState == Emulator::State::RUNNING)
    {
        qDebug() << "[MAINWIN]: Emulator already running";
    }
    else // PAUSED
    {
        m_status.mutex.lock();
        m_status.command = Emulator::Command::STEP;
        m_status.mutex.unlock();

        qDebug() << "[MAINWIN]: Emulator steps";
        success = true;
    }

    return success;
}

bool HbcEmulator::stopCmd()
{
    Emulator::State currentState(getState());
    bool success(false);

    if (currentState == Emulator::State::NOT_INITIALIZED)
    {
        qDebug() << "[MAINWIN]: Emulator is not running";
    }
    else if (currentState == Emulator::State::READY)
    {
        qDebug() << "[MAINWIN]: Emulator is not running";
    }
    else if (currentState == Emulator::State::RUNNING)
    {
        m_status.mutex.lock();
        m_status.command = Emulator::Command::STOP;
        m_status.mutex.unlock();

        wait();

        qDebug() << "[MAINWIN]: Emulator stopped";
        success = true;
    }
    else // PAUSED
    {
        m_status.mutex.lock();
        m_status.command = Emulator::Command::STOP;
        m_status.mutex.unlock();

        wait();

        qDebug() << "[MAINWIN]: Emulator stopped";
        success = true;
    }

    return success;
}

void HbcEmulator::closeEmulator(bool appClosed)
{
    m_status.state = Emulator::State::NOT_INITIALIZED;
    m_status.projectName.clear();

    if (!appClosed)
    {
        //emit statusChanged(m_status.state);
        m_consoleOutput->log("Emulator stopped");
    }

    qDebug() << "[EMULATOR]: Stopped";
}

Emulator::State HbcEmulator::getState()
{
    Emulator::State emulatorState;

    m_status.mutex.lock();
    emulatorState = m_status.state;
    m_status.mutex.unlock();

    return emulatorState;
}

// PRIVATE
HbcEmulator::HbcEmulator(MainWindow *mainWin, Console* consoleOutput)
{
    // Emulator settings
    m_status.state = Emulator::State::NOT_INITIALIZED;
    m_status.projectName = "";

    // Qt settings
    m_consoleOutput = consoleOutput;
    m_mainWindow = mainWin;

    connect(this, SIGNAL(statusChanged(Emulator::State)), m_mainWindow, SLOT(onEmulatorStatusChanged(Emulator::State)), Qt::ConnectionType::BlockingQueuedConnection);
}

void HbcEmulator::run() // Overridden
{
    qDebug() << "[EMULATOR]: Thread started";

    bool stop(false);
    Emulator::State currentState(getState());
    QElapsedTimer timer;

    timer.start();
    while (!stop)
    {
        if (currentState == Emulator::State::RUNNING)
        {
            // TICK COMPUTER
        }

        if (timer.elapsed() >= 100) // in ms
        {
            m_status.mutex.lock();
            currentState = m_status.state;

            if (m_status.state == Emulator::State::NOT_INITIALIZED) // Error
            {
                m_consoleOutput->log("Emulator unexceptedly switched to uninitialized state");
                stop = true;
            }

            // Check commands
            if (m_status.command == Emulator::Command::RUN &&
               (currentState == Emulator::State::PAUSED || m_status.state == Emulator::State::READY))
            {
                m_status.state = Emulator::State::RUNNING;
                //emit statusChanged(m_status.state);

                m_consoleOutput->log("Emulator running at full speed");
            }
            else if (m_status.command == Emulator::Command::STEP &&
                    (currentState == Emulator::State::PAUSED || m_status.state == Emulator::State::READY))
            {
                // TICK COMPUTER
                m_status.command = Emulator::Command::PAUSE;

                qDebug() << "[EMULATOR]: Stepped";
                m_consoleOutput->log("Emulator stepped");
            }
            else if (m_status.command == Emulator::Command::PAUSE && m_status.state == Emulator::State::RUNNING)
            {
                m_status.state = Emulator::State::PAUSED;
                //emit statusChanged(m_status.state);

                qDebug() << "[EMULATOR]: Paused";
                m_consoleOutput->log("Emulator paused");
            }
            else if (m_status.command == Emulator::Command::STOP)
            {
                closeEmulator(false);
                stop = true;
            }
            else if (m_status.command == Emulator::Command::CLOSE)
            {
                closeEmulator(true);
                stop = true;
            }

            currentState = m_status.state;
            m_status.mutex.unlock();

            timer.restart();
        }
    }

    qDebug() << "[EMULATOR]: Thread stopped";
}*/
