#ifndef EEPROM_H
#define EEPROM_H

#include "peripheral.h"
#include "ram.h"

#include <QMutex>

namespace Eeprom
{
    constexpr Byte DEVICE_ID = 0xE1; //!< Random to "look" nice

    constexpr int PORTS_NB = 5;
    enum class Port { CMD = 0, DATA = 1, ADDR_0 = 2, ADDR_1 = 3, ADDR_2 = 4 }; //!< Lists the ports used by the EEPROM device
    enum class Command { NOP = 0, READ = 1, WRITE = 2 }; //<! Lists the commands used by the EEPROM device

    constexpr int MEMORY_SIZE = 0x100000; //<! 1,048,576 bytes (1 MiB)

    struct SafeMemory
    {
        QMutex mutex;

        QByteArray memory;
    };

    /*!
     * \class HbcEeprom
     * \brief Derived from HbcPeripheral, represents the EEPROM device
     *
     * This class is responsible for emulating the HBC-2 EEPROM.
     *
     * <b>Device ID:</b> 0xE1
     *
     * It stores <b>1 MiB of data</b> (20-bit address).
     *
     * On startup, it will dump in RAM:
     * 1. the IVT <i>(0x0100 - 0x02FF)</i>,
     * 2. the 512 first bytes of program memory <i>(0x0300 - 0x04FF)</i>,
     * 3. the interrupt handlers <i>(0xF000 - 0xFFFF)</i>.
     *
     * Port <i>ADDR_0</i> is the <b>most significant byte</b> of the address.
     *
     * If the given address is larger than 20 bits (3 bytes = 24 bits), the 4 most significant bits are ignored.
     *
     * <h2>Control</h2>
     * Like every HbcPeripheral, HbcEeprom uses sockets connecting it to HbcIod ports to communicate with HbcCpu.
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
     *   <td>CMD</td>
     *   <td>Command sent by HbcCpu</td>
     *  </tr>
     *  <tr>
     *   <td>1</td>
     *   <td>DATA</td>
     *   <td>Data read or written</td>
     *  </tr>
     *  <tr>
     *   <td>2</td>
     *   <td>ADDR_0</td>
     *   <td>Most significant byte of the 20-bit address <i>(only the 4 least significant bits are relevant)</i></td>
     *  </tr>
     *  <tr>
     *   <td>3</td>
     *   <td>ADDR_1</td>
     *   <td>Middle byte of the 20-bit address</td>
     *  </tr>
     *  <tr>
     *   <td>4</td>
     *   <td>ADDR_2</td>
     *   <td>Least significant byte of the 20-bit address</td>
     *  </tr>
     * </table>
     *
     * <table>
     *  <caption>List of available commands</caption>
     *  <tr>
     *   <th>ID</th>
     *   <th>Command</th>
     *   <th>Description</th>
     *  </tr>
     *  <tr>
     *   <td>0</td>
     *   <td>NOP</td>
     *   <td>No operation</td>
     *  </tr>
     *  <tr>
     *   <td>1</td>
     *   <td>READ</td>
     *   <td>Reads the byte at the given address on DATA port</td>
     *  </tr>
     *  <tr>
     *   <td>2</td>
     *   <td>WRITE</td>
     *   <td>Writes the byte on DATA port at the given address</td>
     *  </tr>
     * </table>
     *
     * Any invalid command will result in <b>NOP</b>.
     */
    class HbcEeprom : public HbcPeripheral
    {
        public:
            HbcEeprom(QString binaryFilePath, HbcRam *ram, HbcIod *iod, Console *consoleOutput);

            void init() override;
            void tick(bool step) override;

            QByteArray getMemoryContent();

        private:
            bool loadBinaryData();

            SafeMemory m_safeMemory;
            QString m_binaryFilePath;

            HbcRam *m_ram;
    };
}

#endif // EEPROM_H
