#ifndef EMULATOR_H
#define EMULATOR_H

#include <QThread>
#include <QMutex>

//#include "motherboard.h"
//#include "peripheral.h"
#include "console.h"

namespace Emulator
{
    enum class State { NOT_INITIALIZED = 0, READY = 1, RUNNING = 2, PAUSED = 3 };
    enum class Command { NONE = 0, RUN = 1, STEP = 2, PAUSE = 3, STOP = 4, CLOSE = 5 };

    struct Status
    {
        QMutex mutex;

        State state;
        Command command;

        std::string projectName;
    };
}

class MainWindow;

class HbcEmulator : public QThread
{
    Q_OBJECT

    public:
        HbcEmulator(MainWindow *mainWin, Console *consoleOutput);
        ~HbcEmulator();

        bool runCmd();
        bool stepCmd();
        bool pauseCmd();
        bool stopCmd();

        bool loadProject(QByteArray data, std::string projectName);
        bool loadProject(QByteArray data, QString projectName);

        Emulator::State getState();

    signals:
        void statusChanged(Emulator::State newState);

    private:
        void run() override;

        Emulator::Status m_status;

        Console *m_consoleOutput;
        MainWindow *m_mainWindow;
};

/*namespace Emulator
{
    enum class State { NOT_INITIALIZED = 0, READY = 1, RUNNING = 2, PAUSED = 3 };
    enum class Command { RUN = 0, PAUSE = 1, STEP = 2, STOP = 3, CLOSE = 4};

    struct Status
    {
        QMutex mutex;

        State state;
        Command command;

        std::string projectName;
    };

    struct Computer {
        HbcMotherboard motherboard;
        std::vector<HbcPeripheral*> peripherals;
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

    // Emulator commands
    bool runCmd();
    bool stepCmd();
    bool pauseCmd();
    bool stopCmd();

    void closeEmulator(bool appClosed);

    Emulator::State getState();

signals:
    void statusChanged(Emulator::State newState);

private:
    HbcEmulator(Console *consoleOutput);

    void run() override; // Thread loop

    Emulator::Status m_status;

    Console *m_consoleOutput;
};*/

#endif // EMULATOR_H
