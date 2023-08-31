#include "realTimeClock.h"

using namespace RealTimeClock;

// PUBLIC
HbcRealTimeClock::HbcRealTimeClock(HbcIod *iod, Console *consoleOutput) : HbcPeripheral(iod, consoleOutput)
{ }

void HbcRealTimeClock::init()
{
    bool success(true);

    m_sockets = Iod::requestPortsConnexions(*m_iod, RTC_DEVICE_ID, RTC_PORTS_NB);

    if (m_sockets.size() < RTC_PORTS_NB)
    {
        m_consoleOutput->log("Cannot plug the RTC, not enough available ports");

        success = false;
    }

    m_date.setDate(2000, 1, 1); // Default
    m_time.setHMS(0, 0, 0); // Default

    m_clock.start();
}

void HbcRealTimeClock::tick()
{
    if ((Command)*m_sockets[(int)Port::CMD].portDataPointer == Command::GET_TIME)
    {
        putDateTimeOnPorts();
        *m_sockets[(int)Port::CMD].portDataPointer = (int)Command::NOP;
    }
    else if ((Command)*m_sockets[(int)Port::CMD].portDataPointer == Command::SET_TIME)
    {
        int year, month, day, hour, minute, second;
        getDateTimeFromPorts(year, month, day, hour, minute, second);

        *m_sockets[(int)Port::YEAR].portDataPointer = 0; // Fail

        if (QDate::isValid(year, month, day))
        {
            if (QTime::isValid(hour, minute, second))
            {
                m_date.setDate(year, month, day);
                m_time.setHMS(hour, minute, second);

                *m_sockets[(int)Port::YEAR].portDataPointer = 1; // Success
            }
        }
    }
    else
    {
        if (m_clock.elapsed() >= (1000.f / INTERRUPTS_PER_SECOND))
        {
            m_time = m_time.addMSecs(m_clock.elapsed());
            m_clock.restart();

            putDateTimeOnPorts();

            Iod::triggerInterrupt(*m_iod, m_sockets[0].portId);
        }
    }
}

// PRIVATE
void HbcRealTimeClock::putDateTimeOnPorts()
{
    *m_sockets[(int)Port::YEAR].portDataPointer   = m_date.year() - YEAR_0;
    *m_sockets[(int)Port::MONTH].portDataPointer  = m_date.month();
    *m_sockets[(int)Port::DAY].portDataPointer    = m_date.day();
    *m_sockets[(int)Port::HOUR].portDataPointer   = m_time.hour();
    *m_sockets[(int)Port::MINUTE].portDataPointer = m_time.minute();
    *m_sockets[(int)Port::SECOND].portDataPointer = m_time.second();
}

void HbcRealTimeClock::getDateTimeFromPorts(int &year, int &month, int &day, int &hour, int &minute, int &second)
{
    year   = *m_sockets[(int)Port::YEAR].portDataPointer;
    month  = *m_sockets[(int)Port::MONTH].portDataPointer;
    day    = *m_sockets[(int)Port::DAY].portDataPointer;
    hour   = *m_sockets[(int)Port::HOUR].portDataPointer;
    minute = *m_sockets[(int)Port::MINUTE].portDataPointer;
    second = *m_sockets[(int)Port::SECOND].portDataPointer;
}
