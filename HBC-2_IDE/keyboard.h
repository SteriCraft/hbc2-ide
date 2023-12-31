#ifndef KEYBOARD_H
#define KEYBOARD_H

/*!
 * \file keyboard.h
 * \brief Keyboard device derived from HbcPeripheral
 * \author Gianni Leclercq
 * \version 0.1
 * \date 08/09/2023
 */
#include <QKeyEvent>
#include "peripheral.h"

/*!
 * \namespace Keyboard
 *
 * Keyboard device derived from HbcPeripheral
 */
namespace Keyboard
{
    constexpr Byte DEVICE_ID = 0xAA; //!< Random to "look" nice

    constexpr int PORTS_NB = 2;
    enum class Port { PRESSED_SCAN_CODE = 0, RELEASED_SCAN_CODE = 1 }; //!< Lists the ports used by the keyboard device

    const std::map<quint32, Byte> azertyKeyCodeMap = {
    { 0x76, 0x01 },
    { 0x6e, 0x02 },
    { 0x73, 0x03 },
    { 0x70, 0x04 },
    { 0x75, 0x05 },
    { 0x7f, 0x06 },
    { 0x71, 0x07 },
    { 0x16, 0x08 },
    { 0x17, 0x09 },
    { 0x6f, 0x0a },
    { 0x72, 0x0b },
    { 0x74, 0x0c },
    { 0x24, 0x0d },
    { 0x32, 0x0e }, { 0x3e, 0x0e },
    { 0x42, 0x0f },
    { 0x25, 0x10 }, { 0x69, 0x10 },
    { 0x40, 0x11 }, { 0x6c, 0x11 },
    { 0x68, 0x12 },
    { 0x56, 0x13 },
    { 0x52, 0x14 },
    { 0x3f, 0x15 },
    { 0x6a, 0x16 },
    { 0x5b, 0x17 },
    { 0x09, 0x1b },
    { 0x41, 0x20 },
    { 0x3d, 0x21 },
    { 0x0c, 0x22 },
    { 0x23, 0x24 },
    { 0x30, 0x25 },
    { 0x0a, 0x26 },
    { 0x0d, 0x27 },
    { 0x0e, 0x28 },
    { 0x14, 0x29 },
    { 0x33, 0x2a },
    { 0x15, 0x2b },
    { 0x3a, 0x2c },
    { 0x0f, 0x2d },
    { 0x3b, 0x2e },
    { 0x5a, 0x30 },
    { 0x57, 0x31 },
    { 0x58, 0x32 },
    { 0x59, 0x33 },
    { 0x53, 0x34 },
    { 0x54, 0x35 },
    { 0x55, 0x36 },
    { 0x4f, 0x37 },
    { 0x50, 0x38 },
    { 0x51, 0x39 },
    { 0x3c, 0x3a },
    { 0x5e, 0x3c },
    { 0x13, 0x40 },
    { 0x18, 0x41 },
    { 0x38, 0x42 },
    { 0x36, 0x43 },
    { 0x28, 0x44 },
    { 0x1a, 0x45 },
    { 0x29, 0x46 },
    { 0x2a, 0x47 },
    { 0x2b, 0x48 },
    { 0x1f, 0x49 },
    { 0x2c, 0x4a },
    { 0x2d, 0x4b },
    { 0x2e, 0x4c },
    { 0x2f, 0x4d },
    { 0x39, 0x4e },
    { 0x20, 0x4f },
    { 0x21, 0x50 },
    { 0x26, 0x51 },
    { 0x1b, 0x52 },
    { 0x27, 0x53 },
    { 0x1c, 0x54 },
    { 0x1e, 0x55 },
    { 0x37, 0x56 },
    { 0x34, 0x57 },
    { 0x35, 0x58 },
    { 0x1d, 0x59 },
    { 0x19, 0x5a },
    { 0x12, 0x5b },
    { 0x22, 0x5e },
    { 0x11, 0x5f },
    { 0x10, 0x60 },
    { 0x0b, 0x7e },
    { 0x77, 0x7f }
    }; //!< Scan code map for azerty keyboards

    /*!
     * \class HbcKeyboard
     * \brief Derived from HbcPeripheral, represents the Keyboard device
     *
     * This class is responsible for emulating the HBC-2 keyboard.
     *
     * <b>Device ID:</b> 0xAA
     *
     * <h2>Communication</h2>
     * Like every HbcPeripheral, HbcKeyboard uses sockets connecting it to HbcIod ports to send its scan code to the HbcCpu.
     *
     * <table>
     *  <caption>List of available ports</caption>
     *  <tr>
     *   <th>ID</th>
     *   <th>Port</th>
     *   <th>Description</th>
     *  </tr>
     *  <tr>
     *   <td>0</td>
     *   <td>PRESSED_SCAN_CODE</td>
     *   <td>Holds the scan code of the pressed key</td>
     *  </tr>
     *  <tr>
     *   <td>1</td>
     *   <td>RELEASED_SCAN_CODE</td>
     *   <td>Holds the scan code of the pressed key</td>
     *  </tr>
     * </table>
     *
     * <b>AZERTY map</b>
     * \image html keyboard_azerty_map.png
     */
    class HbcKeyboard : public HbcPeripheral
    {
        public:
            HbcKeyboard(HbcIod *iod, Console *consoleOutput);

            void init() override;
            void tick(bool step) override;

            void sendKeyCode(quint32 qtKeyCode, bool release);
    };
}

#endif // KEYBOARD_H
