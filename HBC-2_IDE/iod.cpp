#include "motherboard.h"

void Iod::init(HbcIod &iod, HbcMotherboard *mb)
{
    iod.m_motherboard = mb;

    for (unsigned int i(0); i < PORTS_NB; i++)
    {
        iod.m_ports[i].peripheralId = 0x00;
        iod.m_ports[i].data = 0x00;
    }

    while (iod.m_interruptsQueue.size() > 0)
        iod.m_interruptsQueue.pop();

    qDebug() << "[IOD]: Init"; // TODO: Bug, sometimes no interrupt is thrown on init when relaunching the emulator
}

void Iod::tick(HbcIod &iod)
{
    if (!iod.m_motherboard->m_inr)
    {
        iod.m_motherboard->m_int = iod.m_interruptsQueue.size() > 0;

        /*if (iod.m_motherboard->m_int)
        {
            qDebug() << "[IOD]: " << iod.m_interruptsQueue.size() << " interrupt(s) pending";
        }*/
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
    return iod.m_ports[portId].data;
}

void Iod::setPortData(HbcIod &iod, Byte portId, Byte data)
{
    iod.m_ports[portId].data = data;
}

void Iod::triggerInterrupt(HbcIod &iod, Byte peripheralFirstPortID)
{
    if (iod.m_interruptsQueue.size() < INTERRUPT_QUEUE_SIZE) // If the queue is full, any interrupt is discarded
    {
        Iod::Interrupt newInterrupt;

        newInterrupt.portId = peripheralFirstPortID;
        newInterrupt.data = iod.m_ports[peripheralFirstPortID].data;

        iod.m_interruptsQueue.push(newInterrupt);
    }
}

std::vector<Iod::PortSocket> Iod::requestPortsConnexions(HbcIod &iod, Byte peripheralId, unsigned int nbPortsRequested)
{
    std::vector<PortSocket> sockets;

    for (unsigned int i(1); i < PORTS_NB; i++) // Port #0 is reseved for the IOD to talk to the CPU
    {
        if (iod.m_ports[i].peripheralId == 0) // No peripheral connected
        {
            if (PORTS_NB - i >= nbPortsRequested) // Enough ports available to accept the device
            {
                for (unsigned int j(0); j < nbPortsRequested; j++)
                {
                    PortSocket newSocket;

                    newSocket.portId = i + j;
                    newSocket.portDataPointer = &iod.m_ports[i + j].data;

                    sockets.push_back(newSocket);

                    iod.m_ports[i + j].peripheralId = peripheralId;
                }

                // First interrupt to tell the CPU the first port on which a peripheral was connected
                iod.m_ports[IOD_RESERVED_PORT_NB].data = sockets[0].portId;
                Iod::triggerInterrupt(iod, IOD_RESERVED_PORT_NB);

                // Second interrupt to tell the CPU what peripheral is plugged in
                iod.m_ports[IOD_RESERVED_PORT_NB].data = peripheralId;
                Iod::triggerInterrupt(iod, IOD_RESERVED_PORT_NB);
            }

            break;
        }
    }

    return sockets;
}
