#ifndef MONITOR_H
#define MONITOR_H

#include "peripheral.h"

#define SCREEN_PORTS_NB 5

namespace Monitor
{
    enum class Ports { DATA_0 = 0, DATA_1 = 1, POS_X = 2, POS_Y = 3, CMD = 4 };
    enum class Commands { NOP = 0, WRITE = 1, READ = 2, SWITCH_TO_PIXEL_MODE = 3, SWITCH_TO_TEXT_MODE = 4 };
}

class HbcMonitor : public HbcPeripheral
{
    public:
        HbcMonitor(HbcIod *iod, Console *consoleOutput);
        ~HbcMonitor();

        void init() override;
        void tick() override;

    private:
        bool m_done = false;
};

#endif // MONITOR_H
