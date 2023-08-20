#include "ram.h"

void Ram::write(HbcRam &ram, Word address, Byte data)
{
    ram.m_lock.lock();
    ram.m_memory[address] = data;
    ram.m_lock.unlock();
}

Byte Ram::read(HbcRam &ram, Word address)
{
    Byte valueRead;

    ram.m_lock.lock();
    valueRead = ram.m_memory[address];
    ram.m_lock.unlock();

    return valueRead;
}

bool Ram::setContent(HbcRam &ram, QByteArray data)
{
    if (data.size() == MEMORY_SIZE)
    {
        ram.m_lock.lock();
        for (unsigned int i(0); i < data.size(); i++)
        {
            ram.m_memory[i] = data[i];
        }
        ram.m_lock.unlock();

        return true;
    }
    else
    {
        return false;
    }
}
