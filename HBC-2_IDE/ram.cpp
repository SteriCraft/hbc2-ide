#include "ram.h"

HbcRam* HbcRam::m_singleton = nullptr;


// PUBLIC
HbcRam* HbcRam::getInstance()
{
    if (m_singleton == nullptr)
        m_singleton = new HbcRam();

    return m_singleton;
}

void HbcRam::write(uint16_t address, uint8_t data)
{
    m_memory[address] = data;
}

uint8_t HbcRam::read(uint16_t address)
{
    return m_memory[address];
}

bool HbcRam::setContent(QByteArray data)
{
    if (data.size() == RAM_SIZE)
    {
        for (unsigned int i(0); i < data.size(); i++)
        {
            m_memory[i] = data[i];
        }

        return true;
    }
    else
    {
        return false;
    }
}


// PRIVATE
HbcRam::HbcRam()
{
    for (unsigned int i(0); i < RAM_SIZE; i++)
    {
        m_memory[i] = 0x00;
    }
}
