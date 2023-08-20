#include "peripheral.h"

HbcPeripheral::HbcPeripheral(HbcIod *iod, Console *consoleOutput)
{
    m_ports.clear();

    m_iod = iod;
    m_consoleOutput = consoleOutput;
}

HbcPeripheral::~HbcPeripheral()
{ }

bool HbcPeripheral::sendData(uint8_t portId, uint8_t data)
{
    for (unsigned int i(0); i < m_ports.size(); i++)
    {
        if (m_ports[i].m_portId == portId)
        {
            *m_ports[i].m_portData = data;

            return true;
        }
    }

    return false;
}

bool HbcPeripheral::readData(uint8_t portId, uint8_t &data)
{
    for (unsigned int i(0); i < m_ports.size(); i++)
    {
        if (m_ports[i].m_portId == portId)
        {
            data = *m_ports[i].m_portData;

            return true;
        }
    }

    return false;
}
