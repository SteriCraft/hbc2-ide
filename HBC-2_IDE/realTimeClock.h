#ifndef REALTIMECLOCK_H
#define REALTIMECLOCK_H

/*!
 * \file realTimeClock.h
 * \brief RTC device derived from HbcPeripheral
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QElapsedTimer>
#include "peripheral.h"

/*!
 * \namespace RealTimeClock
 *
 * RTC device derived from HbcPeripheral
 */
namespace RealTimeClock
{
    constexpr Byte RTC_DEVICE_ID = 0x95; //!< Random to "look" nice

    constexpr int RTC_PORTS_NB = 7;
    constexpr int INTERRUPTS_PER_SECOND = 1;
    constexpr int YEAR_0 = 1900; //!< Starts counting from year 1900 to year 2155

    enum class Port { YEAR = 0, MONTH = 1, DAY = 2, HOUR = 3, MINUTE = 4, SECOND = 5, CMD = 6 }; //!< Lists the ports used by the RTC device
    enum class Command { NOP = 0, GET_TIME = 1, SET_TIME = 2 }; //!< Lists the commands for the RTC device

    /*!
     * \class HbcRealTimeClock
     * \brief Derived from HbcPeripheral, represents the RTC device
     *
     * The <b>Real Time Clock</b> (or RTC) is a self-powered device that stores and keeps track of date and time.
     * See <a href="https://doc.qt.io/qt-6/qdate.html">QDate</a> and <a href="https://doc.qt.io/qt-6/qtime.htmll">QTime</a> for more information about date and time storage.
     *
     * <table>
     *  <caption>List of available ports</caption>
     *  <tr>
     *   <th>ID</th>
     *   <th>Port</th>
     *   <th>Description</th>
     *  </tr>
     *  <tr>
     *   <td>0</td>
     *   <td>YEAR</td>
     *   <td>Year value (byte representing [1900 - 2155])</td>
     *  </tr>
     *  <tr>
     *   <td>1</td>
     *   <td>MONTH</td>
     *   <td>Month value</td>
     *  </tr>
     *  <tr>
     *   <td>2</td>
     *   <td>DAY</td>
     *   <td>Day value</td>
     *  </tr>
     *  <tr>
     *   <td>3</td>
     *   <td>HOUR</td>
     *   <td>Hour value</td>
     *  </tr>
     *  <tr>
     *   <td>4</td>
     *   <td>MINUTE</td>
     *   <td>Minute value</td>
     *  </tr>
     *  <tr>
     *   <td>5</td>
     *   <td>SECOND</td>
     *   <td>Second value</td>
     *  </tr>
     *  <tr>
     *   <td>6</td>
     *   <td>CMD</td>
     *   <td>Command sent by the CPU</td>
     *  </tr>
     * </table>
     *
     * <table>
     *  <caption>List of available commands</caption>
     *  <tr>
     *   <th>ID</th>
     *   <th>Command</th>
     *   <th>Description</th>
     *  </tr>
     *  <tr>
     *   <td>0</td>
     *   <td>NOP</td>
     *   <td>No operation</td>
     *  </tr>
     *  <tr>
     *   <td>1</td>
     *   <td>GET_TIME</td>
     *   <td>Sends current date and time on ports</td>
     *  </tr>
     *  <tr>
     *   <td>1</td>
     *   <td>SET_TIME</td>
     *   <td>Sets internal date and time with data available on ports</td>
     *  </tr>
     * </table>
     *
     * Any invalid command will result in <b>NOP</b>.
     */
    class HbcRealTimeClock : public HbcPeripheral
    {
        public:
            /*!
             * \param iod Pointer to the Input/Output Device (IOD).
             * \param consoleOutput Pointer to ConsoleOutput
             */
            HbcRealTimeClock(HbcIod *iod, Console *consoleOutput);

            /*!
             * Initializes the clock's sockets, date and time, and starts the internal clock.
             * Default date and time is 01/01/2000 00:00:00
             *
             * \sa tick()
             */
            void init() override;

            /*!
             * Checks for incoming command on sockets and executes them.
             *
             * \sa init()
             */
            void tick(bool step) override;

        private:
            void putDateTimeOnPorts();
            void getDateTimeFromPorts(int &year, int &month, int &day, int &hour, int &minute, int &second);

            QElapsedTimer m_clock;

            QDate m_date;
            QTime m_time;

            RealTimeClock::Command m_command;
        };
}

#endif // REALTIMECLOCK_H
