#include "motherboard.h"

void Motherboard::tick(HbcMotherboard &motherboard)
{
    Cpu::tick(motherboard.m_cpu);
}

void Motherboard::writeRam(HbcMotherboard &motherboard, uint16_t address, uint8_t data)
{
    Ram::write(motherboard.m_ram, address, data);
}

uint8_t Motherboard::readRam(HbcMotherboard &motherboard, uint16_t address)
{
    return Ram::read(motherboard.m_ram, address);
}

bool Motherboard::init(HbcMotherboard &motherboard, QByteArray data)
{
    if (data.size() == MEMORY_SIZE)
    {
        Ram::setContent(motherboard.m_ram, data);
        Cpu::init(motherboard.m_cpu, &motherboard);

        return true;
    }
    else
    {
        return false;
    }
}
