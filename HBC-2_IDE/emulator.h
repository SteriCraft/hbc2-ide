#ifndef EMULATOR_H
#define EMULATOR_H

#include <thread>
#include <mutex>
#include <QElapsedTimer>

#include "motherboard.h"
#include "console.h"

namespace Emulator
{
    enum class State { STOPPED = 0, RUNNING = 1, PAUSED = 2 };
    enum class Command { STOP = 0, RUN = 1, STEP = 2, PAUSE = 3, EXIT = 4};

    struct Status
    {
        std::mutex m_lock;

        State m_state;
        Command m_command;

        bool isBinarySet() { return m_initialBinary.size() == RAM_SIZE; }
        QByteArray m_initialBinary;
    };

    struct Computer {
        std::mutex m_lock;

        HbcMotherboard *m_motherboard;
    };
}

class HbcEmulator // SINGLETON
{
    static HbcEmulator *m_singleton;

    void mainLoop();

    bool runCmd();
    bool pauseCmd();
    bool stopCmd();
    void exitCmd();

    bool setInitialBinaryData(QByteArray data, std::string projectName);
    bool setInitialBinaryData(QByteArray data, QString projectName);

    Emulator::State getState();

public:
    static HbcEmulator* getInstance(Console *consoleOutput);

private:
    HbcEmulator(Console *consoleOutput);

    std::string m_projectName;
    Emulator::Status m_status;
    Emulator::Computer m_computer;

    Console *m_consoleOutput;
};

#endif // EMULATOR_H
