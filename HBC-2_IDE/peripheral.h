#ifndef PERIPHERAL_H
#define PERIPHERAL_H

#include <cinttypes>
#include <vector>

#include "console.h"

struct HbcMotherboard;
struct HbcIod;

struct PortConnexion
{
    unsigned int m_portId;
    uint8_t *m_portData;
};

class HbcPeripheral
{
    public:
        HbcPeripheral(HbcIod *iod, Console *consoleOutput);
        virtual ~HbcPeripheral() = 0;

        // TODO : Implement monitor with a separate thread

        virtual void tick() = 0;

    protected:
        bool sendData(uint8_t portId, uint8_t data);
        bool readData(uint8_t portId, uint8_t &data);

        std::vector<PortConnexion> m_ports;
        HbcIod *m_iod;
        Console *m_consoleOutput;
};

#endif // PERIPHERAL_H
