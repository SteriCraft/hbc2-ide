#ifndef IOD_H
#define IOD_H

#include <queue>

#include "computerDetails.h"

#define INTERRUPT_QUEUE_SIZE 256

struct HbcMotherboard;

namespace Iod
{
    struct Interrupt
    {
        Byte portId;
        Byte data;
    };

    struct PortSocket
    {
        unsigned int portId;
        uint8_t *portDataPointer;
    };

    struct Port
    {
        unsigned int peripheralId;
        Byte data;
    };
}

struct HbcIod
{
    HbcMotherboard *m_motherboard;

    unsigned int m_nbPeripheralsPlugged;
    Iod::Port m_ports[PORTS_NB]; // <PeripheralId, data>
    std::queue<Iod::Interrupt> m_interruptsQueue;
};

namespace Iod
{
    void init(HbcIod &iod, HbcMotherboard *mb);

    void tick(HbcIod &iod);

    Byte getPortData(HbcIod &iod, Byte portId); // Intended to be used by the processor only
    void setPortData(HbcIod &iod, Byte portId, Byte data);

    void triggerInterrupt(HbcIod &iod, Byte peripheralFirstPortID);

    std::vector<PortSocket> requestPortsConnexions(HbcIod &iod, unsigned int nbPortsRequested);
}

#endif // IOD_H
