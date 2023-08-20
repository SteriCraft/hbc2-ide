#ifndef RAM_H
#define RAM_H

#include <cinttypes>
#include <QByteArray>
#include <QDebug>
#include <QMutex>

#include "computerDetails.h"

struct HbcRam
{
    QMutex m_lock;

    Byte m_memory[MEMORY_SIZE];
};

namespace Ram
{
    void write(HbcRam &ram, Word address, Byte data);
    Byte read(HbcRam &ram, Word address);

    bool setContent(HbcRam &ram, QByteArray data);
}

#endif // RAM_H
