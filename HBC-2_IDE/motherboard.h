#ifndef MOTHERBOARD_H
#define MOTHERBOARD_H

#include <QMutex>

#include "cpu.h"
#include "ram.h"
#include "iod.h"

struct HbcMotherboard
{
    HbcCpu m_cpu;
    HbcRam m_ram;
    HbcIod m_iod;

    Word m_addressBus;
    Byte m_dataBus;

    bool m_int; // Interrupt line
    bool m_inr; // Interrupt ready line
};

namespace Motherboard
{
    void tick(HbcMotherboard &motherboard);

    void writeRam(HbcMotherboard &motherboard, Word address, Byte data);
    uint8_t readRam(HbcMotherboard &motherboard, Word address);

    bool init(HbcMotherboard &motherboard, QByteArray data);
}

#endif // MOTHERBOARD_H
