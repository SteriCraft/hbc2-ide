#ifndef EMULATOR_H
#define EMULATOR_H

/*!
 * \file emulator.h
 * \brief Main code for the HBC-2 Emulator
 * \author Gianni Leclercq
 * \version 0.1
 * \date 28/08/2023
 */
#include <QThread>
#include <QMutex>
#include "motherboard.h"
#include "monitor.h"
#include "realTimeClock.h"
#include "console.h"

/*!
 * \namespace Emulator
 * \brief HbcEmulator specifications.
 */
namespace Emulator
{
    enum class State { NOT_INITIALIZED = 0, READY = 1, RUNNING = 2, PAUSED = 3 }; //!< Lists emulator states
    enum class Command { NONE = 0, RUN = 1, STEP = 2, PAUSE = 3, STOP = 4, CLOSE = 5 }; //!< Lists emulator commands
    enum class FrequencyTarget { KHZ_100 = 100000, MHZ_1 = 1000000, MHZ_2 = 2000000,
                                 MHZ_5 = 5000000, MHZ_10 = 10000000, MHZ_20 = 20000000, FASTEST = 0 }; //!< Lists possible frequency targets

    /*!
     * \struct Status
     * \brief Contains thread safe data to control the emulator
     */
    struct Status
    {
        QMutex mutex;

        State state; //!< Current emulator state
        Command command; //!< Command given to the emulator
        FrequencyTarget frequencyTarget; //!< Selected frequency target

        bool useMonitor; //!< Defined by user before an emulator run
        bool useRTC; //!< Defined by user before an emulator run
        bool useKeyboard; //!< Defined by user before an emulator run
        bool startPaused; //!< Defined by user before an emulator run

        std::string projectName;
    };

    /*!
     * \struct Computer
     * \brief Stores the computer information for the emulator <i>(only used in the emulator thread)</i>
     */
    struct Computer {
        HbcMotherboard motherboard;

        HbcMonitor *monitor;
        RealTimeClock::HbcRealTimeClock *rtc;
        Keyboard::HbcKeyboard *keyboard;
        std::vector<HbcPeripheral*> peripherals; //!< General vector of the peripherals for simpler systemwide iterations

        QByteArray initialRamData; //!< Binary data used on emulator first run
        CpuStatus cpuState; //!< Only updated when the emulator is stopped or when requested
    };
}

class MainWindow;

/*!
 * \class HbcEmulator
 * \brief Where the HBC-2 emulation is done
 *
 * This class is responsible for emulating the HBC-2 CPU and its surrounding components. It uses a separate thread to increase performance.<br>
 * It uses a thread safe struct (Emulator::Status) to be controlled by the main thread.
 *
 * <table>
 *  <caption>List of emulator states</caption>
 *  <tr>
 *   <th>ID</th>
 *   <th>State</th>
 *   <th>Description</th>
 *  </tr>
 *  <tr>
 *   <td>0</td>
 *   <td>NOT_INITIALIZED</td>
 *   <td>No binary data was provided, the thread is not running</td>
 *  </tr>
 *  <tr>
 *   <td>1</td>
 *   <td>READY</td>
 *   <td>Just after binary data was provided, the thread is executed and waiting for Command::RUN</td>
 *  </tr>
 *  <tr>
 *   <td>2</td>
 *   <td>RUNNING</td>
 *   <td>Emulator running at the targeted frequency</td>
 *  </tr>
 *  <tr>
 *   <td>3</td>
 *   <td>PAUSED</td>
 *   <td>Emulator paused during execution</td>
 *  </tr>
 * </table>
 *
 * <table>
 *  <caption>List of available commands</caption>
 *  <tr>
 *   <th>ID</th>
 *   <th>Command</th>
 *   <th>Description</th>
 *   <th>Keyboard shortcut</th>
 *  </tr>
 *  <tr>
 *   <td>0</td>
 *   <td>NONE</td>
 *   <td>No operation</td>
 *   <td></td>
 *  </tr>
 *  <tr>
 *   <td>1</td>
 *   <td>RUN</td>
 *   <td>Starts execution at targeted frequency and sets the emulator's state to RUNNING</td>
 *   <td>F9</td>
 *  </tr>
 *  <tr>
 *   <td>2</td>
 *   <td>STEP</td>
 *   <td>Executes one instruction, does not change the emulator's state</td>
 *   <td>F10</td>
 *  </tr>
 *  <tr>
 *   <td>3</td>
 *   <td>PAUSE</td>
 *   <td>Pauses execution and sets the emulator's state to PAUSED</td>
 *   <td>F11</td>
 *  </tr>
 *  <tr>
 *   <td>4</td>
 *   <td>STOP</td>
 *   <td>Stops execution and sets the emulator's state to READY</td>
 *   <td>F12</td>
 *  </tr>
 *  <tr>
 *   <td>5</td>
 *   <td>CLOSE</td>
 *   <td>Stops the thread and sets the emulator's state to NOT_INITIALIZED</td>
 *  </tr>
 * </table>
 *
 * Any invalid command will result in <b>NONE</b>.
 */
class HbcEmulator : public QThread
{
    Q_OBJECT

    static HbcEmulator *m_singleton;

    public:
        /*!
         * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
         *
         * \param mainWin Pointer to MainWindow
         * \param consoleOutput Pointer to Console
         */
        static HbcEmulator* getInstance(MainWindow *mainWin, Console *consoleOutput);
        ~HbcEmulator();

        /*!
         * \brief Runs the emulator at targeted frequency
         *
         * Sets its state to RUNNING
         *
         * \return <b>true</b> if the command could be executed
         * \return <b>false</b> otherwise
         */
        bool runCmd();

        /*!
         * \brief Steps the emulator
         *
         * Does not affect its state
         *
         * \return <b>true</b> if the command could be executed
         * \return <b>false</b> otherwise
         */
        bool stepCmd();

        /*!
         * \brief Pauses the emulator
         *
         * Sets its state to PAUSED
         *
         * \return <b>true</b> if the command could be executed
         * \return <b>false</b> otherwise
         */
        bool pauseCmd();

        /*!
         * \brief Stops the emulator
         *
         * Sets its state to READY
         *
         * \return <b>true</b> if the command could be executed
         * \return <b>false</b> otherwise
         */
        bool stopCmd();

        bool loadProject(QByteArray initialRamData, std::string projectName);
        bool loadProject(QByteArray initialRamData, QString projectName);

        /*!
         * \return the current content of memory
         */
        const QByteArray getCurrentBinaryData();

        /*!
         * \return the current HbcCpu status
         */
        const CpuStatus getCurrentCpuStatus();

        /*!
         * \return the current value of the program counter
         */
        const Word getCurrentProgramCounter();

        void useMonitor(bool enable);
        void useRTC(bool enable);
        void useKeyboard(bool enable);
        void setStartPaused(bool enable);

        /*!
         * \return current emulator's state
         */
        Emulator::State getState();

        /*!
         * \return current frequency target
         */
        Emulator::FrequencyTarget getFrequencyTarget();

        /*!
         * \return a pointer to the monitor peripheral <i>(<b>nullptr</b> if no monitor plugged in)</i>
         */
        HbcMonitor* getHbcMonitor();

        /*!
         * \return a pointer to the RTC peripheral <i>(<b>nullptr</b> if no RTC plugged in)</i>
         */
        RealTimeClock::HbcRealTimeClock* getHbcRealTimeClock();

        /*!
         * \return a pointer to the keyboard peripheral <i>(<b>nullptr</b> if no keyboard plugged in)</i>
         */
        Keyboard::HbcKeyboard* getHbcKeyboard();

        /*!
         * \param target Desired frequency target
         */
        void setFrequencyTarget(Emulator::FrequencyTarget target);

    signals:
        /*!
         * \brief Emitted whenever the emulator's state changes
         *
         * \param newState New emulator's state
         */
        void statusChanged(Emulator::State newState);

        /*!
         * \brief Emitted wheneven the emulator executes the STEP command
         */
        void stepped();

        /*!
         * \brief Emitted every 100 milliseconds
         *
         * \param countIn100Ms Number of clock cycles executed
         */
        void tickCountSent(int countIn100Ms);

    private:
        HbcEmulator(MainWindow *mainWin, Console *consoleOutput);

        void run() override;

        void initComputer();
        void tickComputer(bool step = false);

        void storeCpuStatus(bool lastState = false);

        Emulator::Status m_status;
        Emulator::Computer m_computer;

        Console *m_consoleOutput;
        MainWindow *m_mainWindow;
};

#endif // EMULATOR_H
