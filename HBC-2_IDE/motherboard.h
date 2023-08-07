#ifndef MOTHERBOARD_H
#define MOTHERBOARD_H

#include <QMutex>

#include "cpu.h"
#include "ram.h"

struct HbcMotherboard
{
    HbcCpu m_cpu;
    HbcRam m_ram;
};

namespace Motherboard
{
    void tick(HbcMotherboard &motherboard);

    void writeRam(HbcMotherboard &motherboard, uint16_t address, uint8_t data);
    uint8_t readRam(HbcMotherboard &motherboard, uint16_t address);

    bool init(HbcMotherboard &motherboard, QByteArray data);
}

#endif // MOTHERBOARD_H
