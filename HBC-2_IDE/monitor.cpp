#include "monitor.h"
#include "iod.h"

#include <QDebug>

HbcMonitor::HbcMonitor(HbcIod *iod, Console *consoleOutput) : HbcPeripheral(iod, consoleOutput)
{
    std::vector<std::pair<unsigned int, Byte*>> portsInfo = Iod::requestPortsConnexions(*m_iod, SCREEN_PORTS_NB);

    if (portsInfo.size() < SCREEN_PORTS_NB)
        m_consoleOutput->log("Cannot plug the monitor, not enough available ports");

    for (unsigned int i(0); i < portsInfo.size(); i++)
    {
        PortConnexion newPortConnexion;
        newPortConnexion.m_portId = portsInfo[i].first;
        newPortConnexion.m_portData = portsInfo[i].second;

        m_ports.push_back(newPortConnexion);
    }
}

HbcMonitor::~HbcMonitor()
{ }

void HbcMonitor::tick()
{
    // TODO
}
