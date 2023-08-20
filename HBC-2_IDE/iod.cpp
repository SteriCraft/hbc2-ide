#include "motherboard.h"

void Iod::init(HbcIod &iod, HbcMotherboard *mb)
{
    iod.m_motherboard = mb;

    iod.m_nbPeripheralsPlugged = 0;

    for (unsigned int i(0); i < PORTS_NB; i++)
    {
        iod.m_ports[i].first = 0x00;
        iod.m_ports[i].second = 0x00;
    }

    while (iod.m_interruptsQueue.size() > 0)
        iod.m_interruptsQueue.pop();
}

void Iod::tick(HbcIod &iod)
{
    if (!iod.m_motherboard->m_inr)
    {
        iod.m_motherboard->m_int = iod.m_interruptsQueue.size() > 0;
        qDebug() << "[IOD]: Interrupt(s) pending";
    }
    else
    {
        iod.m_motherboard->m_int = false;

        if (iod.m_interruptsQueue.size() > 0)
        {
            iod.m_motherboard->m_addressBus = iod.m_interruptsQueue.front().portId;
            iod.m_motherboard->m_dataBus = iod.m_interruptsQueue.front().data;

            iod.m_interruptsQueue.pop();
        }
    }
}

Byte Iod::getPortData(HbcIod &iod, Byte portId)
{
    return iod.m_ports[portId].second;
}

void Iod::setPortData(HbcIod &iod, Byte portId, Byte data)
{
    iod.m_ports[portId].second = data;
}

std::vector<std::pair<unsigned int, Byte*>> Iod::requestPortsConnexions(HbcIod &iod, unsigned int nbPortsRequested)
{
    std::vector<std::pair<unsigned int, Byte*>> portsPointers;

    for (unsigned int i(0); i < PORTS_NB; i++)
    {
        if (iod.m_ports[i].first == 0) // No peripheral plugged
        {
            if (PORTS_NB - i >= nbPortsRequested)
            {
                iod.m_nbPeripheralsPlugged++;

                for (unsigned int j(0); j < nbPortsRequested; j++)
                {
                    std::pair<unsigned int, Byte*> portConnexionInfo;

                    portConnexionInfo.first = i + j;
                    portConnexionInfo.second = &iod.m_ports[i + j].second;

                    portsPointers.push_back(portConnexionInfo);

                    iod.m_ports[i + j].first = iod.m_nbPeripheralsPlugged;
                }
            }

            break;
        }
    }

    return portsPointers;
}
