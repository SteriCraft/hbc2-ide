#include "motherboard.h"

HbcMotherboard* HbcMotherboard::m_singleton = nullptr;


// PUBLIC
HbcMotherboard* HbcMotherboard::getInstance()
{
    if (m_singleton == nullptr)
        m_singleton = new HbcMotherboard();

    return m_singleton;
}

void HbcMotherboard::writeRam(uint16_t address, uint8_t data)
{
    m_ram->write(address, data);
}

uint8_t HbcMotherboard::readRam(uint16_t address)
{
    return m_ram->read(address);
}


// PRIVATE
HbcMotherboard::HbcMotherboard()
{
    m_cpu = HbcCpu::getInstance(this);
}
