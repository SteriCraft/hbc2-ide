#include "ram.h"

void Ram::write(HbcRam &ram, Word address, Byte data)
{
    ram.mutex.lock();
    ram.memory[address] = data;
    ram.mutex.unlock();
}

Byte Ram::read(HbcRam &ram, Word address)
{
    Byte valueRead;

    ram.mutex.lock();
    valueRead = ram.memory[address];
    ram.mutex.unlock();

    return valueRead;
}

bool Ram::setContent(HbcRam &ram, QByteArray data)
{
    if (data.size() == MEMORY_SIZE)
    {
        ram.mutex.lock();
        for (unsigned int i(0); i < data.size(); i++)
        {
            ram.memory[i] = data[i];
        }
        ram.mutex.unlock();

        return true;
    }
    else
    {
        return false;
    }
}

void Ram::fillNull(HbcRam &ram)
{
    ram.mutex.lock();
    for (unsigned int i(0); i < MEMORY_SIZE; i++)
    {
        ram.memory[i] = 0x00;
    }
    ram.mutex.unlock();
}
