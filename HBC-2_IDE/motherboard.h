#ifndef MOTHERBOARD_H
#define MOTHERBOARD_H

/*!
 * \file motherboard.h
 * \brief HBC-2 Motherboard implementation
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QMutex>
#include "cpu.h"
#include "ram.h"
#include "iod.h"

/*!
 * \struct HbcMotherboard
 * \brief Stores the Motherboard state
 */
struct HbcMotherboard
{
    HbcCpu m_cpu;
    HbcRam m_ram;
    HbcIod m_iod;

    Word m_addressBus; //!< 16-bit
    Byte m_dataBus; //!< 8-bit

    bool m_int; //!< INT: Interrupt signal
    bool m_inr; //!< INR: Interrupt Ready signal
};

/*!
 * \namespace Motherboard
 *
 * Motherboard specifications.<br>
 * Find more info in the CPU documentation.
 */
namespace Motherboard
{
    /*!
     * \brief Initializes the motherboard
     *
     * \param data 65,536 bytes of binary data
     * \return <b>false</b> if the given binary data size is not equal to Ram::MEMORY_SIZE
     */
    bool init(HbcMotherboard &motherboard, QByteArray data);

    /*!
     * \brief Executes one clock cycle through the motherboard
     *
     * 1. HbcCpu tick
     * 2. HbcIod tick
     *
     * \param motherboard
     */
    void tick(HbcMotherboard &motherboard);

    /*!
     * \brief Writes HbcRam memory
     *
     * \param address 16-bit address
     * \param data 8-bit data
     */
    void writeRam(HbcMotherboard &motherboard, Word address, Byte data);

    /*!
     * \brief Reads HbcRam memory
     *
     * \param address 16-bit address
     * \return 8-bit data
     */
    uint8_t readRam(HbcMotherboard &motherboard, Word address);
}

#endif // MOTHERBOARD_H
