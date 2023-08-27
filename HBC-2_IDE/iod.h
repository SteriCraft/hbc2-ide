#ifndef IOD_H
#define IOD_H

/*!
 * \file iod.h
 * \brief Defines the Input/Output Device
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <queue>
#include "computerDetails.h"

struct HbcMotherboard;

/*!
 * \struct HbcIod
 * \brief Stores the Iod state
 */
struct HbcIod
{
    HbcMotherboard *m_motherboard; //!< Used internaly

    unsigned int m_nbPeripheralsPlugged; //*! Number of peripherals plugged in
    Iod::Port m_ports[Iod::PORTS_NB]; //*! Input/Output Device ports
    std::queue<Iod::Interrupt> m_interruptsQueue; //!< Queue size is defined by INTERRUPT_QUEUE_SIZE
};

namespace Iod
{
    void init(HbcIod &iod, HbcMotherboard *mb);

    void tick(HbcIod &iod);

    /*!
     * \brief Returns data available on a port of HbcIod
     *
     * <b>WARNING:</b> Intended to be used by HbcCpu only
     *
     * \param portId ID of the port (must be inferior to PORTS_NB)
     * \return data available on the port
     */
    Byte getPortData(HbcIod &iod, Byte portId);

    /*!
     * \brief Sets data on a port of HbcIod
     *
     * Can be equally be used by HbcCpu or a HbcPeripheral
     *
     * \param portId ID of the port (must be inferior to PORTS_NB)
     */
    void setPortData(HbcIod &iod, Byte portId, Byte data);

    /*!
     * \brief Triggers an interrupt for HbcCpu
     *
     * Stacks interrupt in a queue (of size INTERRUPT_QUEUE_SIZE, see Iod), until HbcCpu handles it
     * <b>WARNING:</b> Intended to be used by HbcPeripheral only
     *
     * \param peripheralFirstPortID ID of the first port to which the sender peripheral is plugged into
     */
    void triggerInterrupt(HbcIod &iod, Byte peripheralFirstPortID);

    /*!
     * \brief Requests HbcIod for available ports
     *
     * HbcIod will keep track of the peripheral id plugged in the ports <i><b>(under development)</b></i><br>
     * <i>Returns an <b>empty vector of sockets</b> if there is not enough available ports</i>
     *
     * \param nbPortsRequested Number of ports requested
     * \return a vector of the sockets created <b>(empty if there is not enough available ports)</b>
     */
    std::vector<PortSocket> requestPortsConnexions(HbcIod &iod, unsigned int nbPortsRequested);
}

#endif // IOD_H
