#include "monitor.h"
#include "iod.h"

#include <QDebug>

HbcMonitor::HbcMonitor(HbcIod *iod, Console *consoleOutput) : HbcPeripheral(iod, consoleOutput)
{ }

HbcMonitor::~HbcMonitor()
{ }

void HbcMonitor::init()
{
    m_sockets = Iod::requestPortsConnexions(*m_iod, SCREEN_PORTS_NB);

    if (m_sockets.size() < SCREEN_PORTS_NB)
        m_consoleOutput->log("Cannot plug the monitor, not enough available ports");
}

void HbcMonitor::tick()
{
    // TODO

    if (!m_done)
    {
        qDebug() << "[MONITOR]: Triggers interrupt on port #" << m_sockets[1].portId;
        *m_sockets[1].portDataPointer = 0x15;
        Iod::triggerInterrupt(*m_iod, m_sockets[1].portId);

        m_done = true;
    }
}
