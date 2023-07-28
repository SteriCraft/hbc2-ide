#ifndef MOTHERBOARD_H
#define MOTHERBOARD_H

#include "cpu.h"
#include "ram.h"

class HbcMotherboard // SINGLETON
{
    static HbcMotherboard *m_singleton;

public:
    static HbcMotherboard* getInstance();

    void writeRam(uint16_t address, uint8_t data);
    uint8_t readRam(uint16_t address);

private:
    HbcMotherboard();

    HbcCpu *m_cpu;
    HbcRam *m_ram;
};

#endif // MOTHERBOARD_H
