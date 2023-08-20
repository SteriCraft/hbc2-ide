#ifndef EMULATOR_H
#define EMULATOR_H

#include <QThread>
#include <QMutex>

#include "motherboard.h"
#include "peripheral.h"
#include "console.h"

namespace Emulator
{
    enum class State { RUNNING = 1, PAUSED = 2, NOT_INITIALIZED = 3 };
    enum class Command { RUN = 0, STEP = 1, PAUSE = 2, CLOSE = 4, CLOSE_APP = 5, NONE = 6};

    struct Status
    {
        QMutex m_lock;

        State m_state;
        Command m_command;

        bool m_plugMonitor;

        std::string m_projectName;
    };

    struct Computer {
        HbcMotherboard m_motherboard;
        std::vector<HbcPeripheral*> m_peripherals;
    };
}

class MainWindow;

class HbcEmulator : public QThread // SINGLETON
{
    Q_OBJECT

    static HbcEmulator *m_singleton;

public:
    static HbcEmulator* getInstance(MainWindow *mainWin, Console *consoleOutput);
    ~HbcEmulator();

    void run() override; // Thread loop

    // Emulator commands
    bool runCmd();
    bool stepCmd();
    bool pauseCmd();
    bool stopCmd();

    bool loadProject(QByteArray data, std::string projectName);
    bool loadProject(QByteArray data, QString projectName);

    void setMonitorPlugged(bool monitor);

    Emulator::State getState();

signals:
    void statusChanged(Emulator::State newState);

private:
    HbcEmulator(MainWindow *mainWin, Console *consoleOutput);

    void tickComputer();
    void checkStatusChange();

    Emulator::Status m_status;
    Emulator::Computer m_computer;

    Console *m_consoleOutput;
    MainWindow *m_mainWindow;
};

#endif // EMULATOR_H
