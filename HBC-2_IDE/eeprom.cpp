#include "eeprom.h"

#include <QFile>

using namespace Eeprom;

HbcEeprom::HbcEeprom(QString binaryFilePath, HbcRam *ram, HbcIod *iod, Console *consoleOutput) : HbcPeripheral(iod, consoleOutput)
{
    m_binaryFilePath = binaryFilePath;
    m_ram = ram;
}

void HbcEeprom::init()
{
    m_sockets = Iod::requestPortsConnexions(*m_iod, DEVICE_ID, PORTS_NB);

    if (m_sockets.size() < PORTS_NB)
    {
        m_consoleOutput->log("Cannot plug the EEPROM, not enough available ports");
    }

    // Copies the IVT (0x100-0x1FF), 512 first byte of program (0x300-0x4FF) and interrupt handlers (0xF000-0xFFFF)
    if (loadBinaryData())
    {
        m_ram->mutex.lock();        
        for (Word i(0x100); i < 0x500; i++)
        {
            m_ram->memory[i] = m_safeMemory.memory[i];
        }

        for (Dword i(0xf000); i < 0x10000; i++)
        {
            m_ram->memory[(Word)i] = m_safeMemory.memory[i];
        }
        m_ram->mutex.unlock();
    }
}

void HbcEeprom::tick(bool step)
{
    //qDebug() << "tick locks";
    m_safeMemory.mutex.lock();
    if (m_safeMemory.memory.size() == MEMORY_SIZE)
    {
        Command command = (Command)*m_sockets[(int)Port::CMD].portDataPointer;
        Dword address = 0x00000000;

        address += *m_sockets[(int)Port::ADDR_2].portDataPointer;
        address += ((int)*m_sockets[(int)Port::ADDR_1].portDataPointer) << 8;
        address += ((int)*m_sockets[(int)Port::ADDR_0].portDataPointer) << 16;
        address &= 0xFFFFF;

        if (command == Command::READ)
        {
            *m_sockets[(int)Port::DATA].portDataPointer = m_safeMemory.memory[address];
        }
        else if (command == Command::WRITE)
        {
            m_safeMemory.memory[address] = *m_sockets[(int)Port::DATA].portDataPointer;
        }
    }
    m_safeMemory.mutex.unlock();
    //qDebug() << "tick unlocks";
}

QByteArray HbcEeprom::getMemoryContent()
{
    QByteArray exportContent;

    //qDebug() << "getMemoryContent locks";
    m_safeMemory.mutex.lock();
    exportContent = m_safeMemory.memory;
    m_safeMemory.mutex.unlock();
    //qDebug() << "getMemoryContent unlocks";

    return exportContent;
}

// PRIVATE
bool HbcEeprom::loadBinaryData()
{
    //qDebug() << "loadBinaryData locks";
    bool success = false;
    QFile binaryFile(m_binaryFilePath);

    m_safeMemory.mutex.lock();
    m_safeMemory.memory.clear();

    if (QFile::exists(m_binaryFilePath))
    {
        if (binaryFile.open(QIODevice::ReadOnly))
        {
            m_safeMemory.memory = binaryFile.readAll();

            if (m_safeMemory.memory.size() != MEMORY_SIZE)
            {
                m_consoleOutput->log("The binary file for the EEPROM is the wrong size (1'048'575 bytes)");
            }
            else
            {
                m_consoleOutput->log("EEPROM binary file successfuly loaded");
                success = true;
            }
        }
    }
    else
    {
        m_consoleOutput->log("The binary file for the EEPROM could not be opened");
    }

    if (!success)
    {
        quint8 nullChar(0);
        m_safeMemory.memory = QByteArray(MEMORY_SIZE, nullChar);
    }
    m_safeMemory.mutex.unlock();
    //qDebug() << "loadBinaryData unlocks";

    return success;
}
