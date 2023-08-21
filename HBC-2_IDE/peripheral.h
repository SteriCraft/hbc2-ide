#ifndef PERIPHERAL_H
#define PERIPHERAL_H

#include <cinttypes>
#include <vector>

#include "iod.h"
#include "console.h"

struct HbcMotherboard;

class HbcPeripheral
{
    public:
        HbcPeripheral(HbcIod *iod, Console *consoleOutput);
        virtual ~HbcPeripheral() = 0;

        virtual void init() = 0;
        virtual void tick() = 0;

    protected:
        bool sendData(Iod::PortSocket socket, Byte data);
        Byte readData(Iod::PortSocket socket);

        std::vector<Iod::PortSocket> m_sockets;
        HbcIod *m_iod;
        Console *m_consoleOutput;
};

#endif // PERIPHERAL_H
