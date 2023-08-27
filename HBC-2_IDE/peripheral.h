#ifndef PERIPHERAL_H
#define PERIPHERAL_H

/*!
 * \file peripheral.h
 * \brief HBC-2 Peripheral implementation
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <cinttypes>
#include <vector>
#include "iod.h"
#include "console.h"

struct HbcMotherboard;

/*!
 * \brief Virtual class used to derive new peripherals
 *
 * Implements minimum functionalities common to every peripheral
 */
class HbcPeripheral
{
    public:
    /*!
         * \brief HbcPeripheral
         * \param consoleOutput Pointer to MainWindow's console output
         */
        HbcPeripheral(HbcIod *iod, Console *consoleOutput);
        virtual ~HbcPeripheral() = 0; //*! Must be overriden to destroy the derived peripheral

        virtual void init() = 0; //*! Must be overriden to initialize the derived peripheral
        virtual void tick() = 0; //*! Must be overriden to tick the derived peripheral

    protected:
        /*!
         * \brief Send data to HbcIod through a socket
         * \param socket Selected socket
         * \param data Data to send
         * \return <b>false</b> if the given socket is not one provided by HbcIod (HbcPeripheral::m_sockets)
         */
        bool sendData(Iod::PortSocket socket, Byte data);

        /*!
         * \brief Read data from HbcIod through a socket
         *
         * Returns <b>0x00</b> if the given socket is not one provided by HbcIod (HbcPeripheral::m_sockets)
         *
         * \param socket Selected socket
         */
        Byte readData(Iod::PortSocket socket);

        std::vector<Iod::PortSocket> m_sockets; //!< Sockets to allocated ports by HbcIod on init()
        HbcIod *m_iod;
        Console *m_consoleOutput;
};

#endif // PERIPHERAL_H
