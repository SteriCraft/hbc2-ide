#ifndef IOD_H
#define IOD_H

#include <queue>

#include "computerDetails.h"

struct HbcMotherboard;

struct Interrupt
{
    Byte portId;
    Byte data;
};

struct HbcIod
{
    HbcMotherboard *m_motherboard;

    unsigned int m_nbPeripheralsPlugged;
    std::pair<unsigned int, Byte> m_ports[PORTS_NB]; // <PeripheralId, data>
    std::queue<Interrupt> m_interruptsQueue;
};

namespace Iod
{
    void init(HbcIod &iod, HbcMotherboard *mb);

    void tick(HbcIod &iod);

    Byte getPortData(HbcIod &iod, Byte portId);
    void setPortData(HbcIod &iod, Byte portId, Byte data);

    std::vector<std::pair<unsigned int, Byte*>> requestPortsConnexions(HbcIod &iod, unsigned int nbPortsRequested); // <PortId, data pointer>
}

#endif // IOD_H
