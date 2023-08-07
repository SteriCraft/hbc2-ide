#ifndef RAM_H
#define RAM_H

#include <cinttypes>
#include <QByteArray>
#include <QDebug>

#include "cpuDetails.h"

struct HbcRam
{
    uint8_t m_memory[MEMORY_SIZE];
};

namespace Ram
{
    void write(HbcRam &ram, uint16_t address, uint8_t data);
    uint8_t read(HbcRam &ram, uint16_t address);

    bool setContent(HbcRam &ram, QByteArray data);
}

#endif // RAM_H
