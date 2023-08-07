#include "ram.h"

void Ram::write(HbcRam &ram, uint16_t address, uint8_t data)
{
    ram.m_memory[address] = data;
}

uint8_t Ram::read(HbcRam &ram, uint16_t address)
{
    return ram.m_memory[address];
}

bool Ram::setContent(HbcRam &ram, QByteArray data)
{
    if (data.size() == MEMORY_SIZE)
    {
        for (unsigned int i(0); i < data.size(); i++)
        {
            ram.m_memory[i] = data[i];
        }

        return true;
    }
    else
    {
        return false;
    }
}
