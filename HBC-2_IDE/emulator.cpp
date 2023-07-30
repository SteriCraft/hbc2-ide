#include "emulator.h"

HbcEmulator* HbcEmulator::m_singleton = nullptr;

// PUBLIC
HbcEmulator* HbcEmulator::getInstance(Console *consoleOutput)
{
    if (m_singleton == nullptr)
        m_singleton = new HbcEmulator(consoleOutput);

    return m_singleton;
}

void HbcEmulator::mainLoop()
{
    const std::lock_guard<std::mutex> lock(m_computer.m_lock);

    bool exists = true;
    Emulator::State currentStatus = Emulator::State::STOPPED; // As method is ran just after creation of the object, its status is necessarily STOPPED

    QElapsedTimer timer;

    timer.start();
    while (exists)
    {
        if (currentStatus == Emulator::State::RUNNING)
        {
            m_computer.m_motherboard->tick();
        }

        if (timer.elapsed() == 100) // in ms
        {
            m_status.m_lock.lock();

            switch (m_status.m_command)
            {
            case Emulator::Command::STOP:
                m_status.m_state = Emulator::State::STOPPED;

                m_computer.m_motherboard->init(m_status.m_initialBinary);
                break;

            case Emulator::Command::RUN:
                m_status.m_state = Emulator::State::RUNNING;
                break;

            case Emulator::Command::STEP:
                m_computer.m_motherboard->tick();
                break;

            case Emulator::Command::PAUSE:
                m_status.m_state = Emulator::State::PAUSED;
                break;

            case Emulator::Command::EXIT:
                exists = false;
                break;
            }

            m_status.m_lock.unlock();
        }
    }

    // Method exited if main thread asked to close
    // The destructor will be called just after
}

bool HbcEmulator::runCmd()
{
    bool success = false;

    m_status.m_lock.lock();

    if (m_status.m_state == Emulator::State::STOPPED || m_status.m_state == Emulator::State::PAUSED)
    {
        if (m_status.isBinarySet())
        {
            m_status.m_command = Emulator::Command::RUN;
            success = true;

            m_consoleOutput->log("Running project \"" + m_projectName + "\" in emulator");
            m_consoleOutput->returnLine();
        }
        else
        {
            m_consoleOutput->log("Couldn't run the emulator because initial binary data is not set");
            m_consoleOutput->returnLine();
        }
    }
    else
    {
        m_consoleOutput->log("Couldn't run the emulator because it is not paused or stopped");
    }

    m_status.m_lock.unlock();

    return success;
}

bool HbcEmulator::pauseCmd()
{
    bool success = false;

    m_status.m_lock.lock();

    if (m_status.m_state == Emulator::State::RUNNING)
    {
        m_status.m_command = Emulator::Command::PAUSE;
        success = true;
    }
    else
    {
        m_consoleOutput->log("Couldn't pause the emulator because it is not running");
        m_consoleOutput->returnLine();
    }

    m_status.m_lock.unlock();

    return success;
}

bool HbcEmulator::stopCmd()
{
    bool success = false;

    m_status.m_lock.lock();

    if (m_status.m_state == Emulator::State::RUNNING || m_status.m_state == Emulator::State::PAUSED)
    {
        m_status.m_command = Emulator::Command::STOP;
        success = true;
    }
    else
    {
        m_consoleOutput->log("Couldn't stop the emulator because it is not running or paused");
        m_consoleOutput->returnLine();
    }

    m_status.m_lock.unlock();

    return success;
}

void HbcEmulator::exitCmd()
{
    m_status.m_lock.lock();

    m_status.m_command = Emulator::Command::EXIT;

    m_status.m_lock.unlock();
}

bool HbcEmulator::setInitialBinaryData(QByteArray data, std::string projectName)
{
    bool success = false;

    m_status.m_lock.lock();

    if (m_status.m_state == Emulator::State::STOPPED)
    {
        if (data.size() == RAM_SIZE)
        {
            m_status.m_initialBinary = data;
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
        m_consoleOutput->log("Couldn't set initial binary data in the emulator because it's running or paused");
        m_consoleOutput->returnLine();
    }

    m_status.m_lock.unlock();

    return success;
}

bool HbcEmulator::setInitialBinaryData(QByteArray data, QString projectName)
{
    return setInitialBinaryData(data, projectName.toStdString());
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
HbcEmulator::HbcEmulator(Console* consoleOutput)
{
    m_status.m_state = Emulator::State::STOPPED;

    m_projectName = "";
    m_computer.m_motherboard = HbcMotherboard::getInstance();

    m_consoleOutput = consoleOutput;
}
