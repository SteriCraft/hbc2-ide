#ifndef RAM_H
#define RAM_H

/*!
 * \file ram.h
 * \brief HBC-2 Random Access Memory implementation
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <cinttypes>
#include <QByteArray>
#include <QDebug>
#include <QMutex>
#include "computerDetails.h"

/*!
 * \struct HbcRam
 * \brief Stores the Ram state
 */
struct HbcRam
{
    QMutex mutex;
    Byte memory[Ram::MEMORY_SIZE]; //!< 65,536 bytes
};

// Already documented in computerDetails.h
namespace Ram
{
    /*!
     * \brief Writes data in memory
     * \param address 16-bit address
     * \param data 8-bit data
     */
    void write(HbcRam &ram, Word address, Byte data);

    /*!
     * \brief Reads data in memory
     * \param address 16-bit address
     *
     * \return 8-bit data
     */
    Byte read(HbcRam &ram, Word address);

    /*!
     * \brief Sets memory content
     * \param data 65,536 bytes of binary data
     * \return <b>false</b> if the given binary data size is not equal to Ram::MEMORY_SIZE
     */
    bool setContent(HbcRam &ram, QByteArray data);

    /*!
     * \brief Fills the memory with value <b>0x00</b>
     */
    void fillNull(HbcRam &ram);
}

#endif // RAM_H
