#ifndef MONITOR_H
#define MONITOR_H

#include "peripheral.h"

#define SCREEN_PORTS_NB 5

class HbcMonitor : public HbcPeripheral
{
public:
    HbcMonitor(HbcIod *iod, Console *consoleOutput);
    ~HbcMonitor();

    void tick() override;

private:
};

#endif // MONITOR_H
