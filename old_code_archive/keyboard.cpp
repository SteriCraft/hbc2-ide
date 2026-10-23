#include "keyboard.h"

using namespace Keyboard;

HbcKeyboard::HbcKeyboard(HbcIod *iod, Console *consoleOutput) : HbcPeripheral(iod, consoleOutput)
{ }

void HbcKeyboard::init()
{
    m_sockets = Iod::requestPortsConnexions(*m_iod, DEVICE_ID, PORTS_NB);

    if (m_sockets.size() < PORTS_NB)
    {
        m_consoleOutput->log("Cannot plug the keyboard, not enough available ports");
    }
}

void HbcKeyboard::tick(bool step)
{ }

void HbcKeyboard::sendKeyCode(quint32 qtKeyCode, bool release)
{
    if (azertyKeyCodeMap.find(qtKeyCode) != azertyKeyCodeMap.end())
    {
        Byte keyCode(azertyKeyCodeMap.at(qtKeyCode));

        if (release)
        {
            *m_sockets[(int)Port::RELEASED_SCAN_CODE].portDataPointer = keyCode;
            Iod::triggerInterrupt(*m_iod, m_sockets[(int)Port::RELEASED_SCAN_CODE].portId);
        }
        else
        {
            *m_sockets[(int)Port::PRESSED_SCAN_CODE].portDataPointer = keyCode;
            Iod::triggerInterrupt(*m_iod, m_sockets[(int)Port::PRESSED_SCAN_CODE].portId);
        }
    }
}
