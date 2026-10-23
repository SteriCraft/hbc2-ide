#include "motherboard.h"

bool Motherboard::init(HbcMotherboard &motherboard, QByteArray ramData)
{
    motherboard.m_addressBus = 0;
    motherboard.m_dataBus = 0;

    motherboard.m_int = false;
    motherboard.m_inr = false;

    if (ramData.size() == Ram::MEMORY_SIZE) // RAM initial binary data provided
    {
        Ram::setContent(motherboard.m_ram, ramData);
        Cpu::init(motherboard.m_cpu, &motherboard);
        Iod::init(motherboard.m_iod, &motherboard);
    }
    else if (ramData.size() == 0) // RAM initial binary data not provided, using the EEPROM
    {
        Ram::fillNull(motherboard.m_ram);
        Cpu::init(motherboard.m_cpu, &motherboard);
        Iod::init(motherboard.m_iod, &motherboard);
    }
    else
        return false;

    return true;
}

void Motherboard::tick(HbcMotherboard &motherboard)
{
    Cpu::tick(motherboard.m_cpu);
    Iod::tick(motherboard.m_iod);
}

void Motherboard::writeRam(HbcMotherboard &motherboard, uint16_t address, uint8_t data)
{
    Ram::write(motherboard.m_ram, address, data);
}

uint8_t Motherboard::readRam(HbcMotherboard &motherboard, uint16_t address)
{
    return Ram::read(motherboard.m_ram, address);
}
