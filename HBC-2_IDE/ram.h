#ifndef RAM_H
#define RAM_H

#include <cinttypes>
#include <QByteArray>
#include <QDebug>

#define RAM_SIZE 0x10000 // 65'536 bytes

class HbcRam // SINGLETON
{
    static HbcRam *m_singleton;

public:
    static HbcRam* getInstance();

    void write(uint16_t address, uint8_t data);
    uint8_t read(uint16_t address);

    bool setContent(QByteArray data);

private:
    HbcRam();

    uint8_t m_memory[RAM_SIZE];
};

#endif // RAM_H
