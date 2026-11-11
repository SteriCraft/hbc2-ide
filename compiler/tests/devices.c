/* Expected: main returns 0 (devices found by class, from INFO) */
/* Returns the number of the first failed check. The emulator's I/O
   processor has a monitor (ports 1-5), an RTC (6-12), a keyboard (13-14)
   and an EEPROM (15-19). */

#include <hbc2.h>

int main(void)
{
    const struct device *d;

    if (io_protocol() != 1)
        return 1;
    if (device_count() != 4)
        return 2;

    d = device_find(DEVICE_INPUT, DEVICE_KEYBOARD, 0);     /* the first keyboard */
    if (d == 0 || d->first_port != 13 || d->ports != 2)
        return 3;
    if (d->product != PRODUCT_KEYBOARD)
        return 4;
    if (device_find(DEVICE_INPUT, DEVICE_KEYBOARD, 1) != 0) /* no second one */
        return 5;

    d = device_find(DEVICE_DISPLAY, DEVICE_ANY, 0);
    if (d == 0 || d->base_class != DEVICE_DISPLAY || d->subclass != DEVICE_VGA)
        return 6;
    if (d->first_port != monitor_port)
        return 7;

    d = device_find(DEVICE_STORAGE, DEVICE_OTHER, 0);
    if (d == 0 || d->product != PRODUCT_EEPROM || d->first_port != eeprom_port)
        return 8;
    if (eeprom_port != 15)
        return 9;

    if (device_find(DEVICE_NETWORK, DEVICE_ANY, 0) != 0)
        return 10;

    d = device_get(1);                                      /* in port order */
    if (d == 0 || d->base_class != DEVICE_SYSTEM || d->subclass != DEVICE_RTC)
        return 11;
    if (d->first_port != rtc_port || rtc_port != 6)
        return 12;
    if (device_get(4) != 0)
        return 13;

    if (keyboard_port != 13 || monitor_port != 1)
        return 14;
    return 0;
}
