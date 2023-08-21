#include "peripheral.h"

HbcPeripheral::HbcPeripheral(HbcIod *iod, Console *consoleOutput)
{
    m_sockets.clear();

    m_iod = iod;
    m_consoleOutput = consoleOutput;
}

HbcPeripheral::~HbcPeripheral()
{ }

bool HbcPeripheral::sendData(Iod::PortSocket socket, Byte data)
{
    for (unsigned int i(0); i < m_sockets.size(); i++)
    {
        if (m_sockets[i].portId == socket.portId)
        {
            *m_sockets[i].portDataPointer = data;
            return true;
        }
    }

    return false;
}

Byte HbcPeripheral::readData(Iod::PortSocket socket)
{
    Byte data(0x00);

    for (unsigned int i(0); i < m_sockets.size(); i++)
    {
        if (m_sockets[i].portId == socket.portId)
        {
            data = *m_sockets[i].portDataPointer;
            break;
        }
    }

    return data;
}
