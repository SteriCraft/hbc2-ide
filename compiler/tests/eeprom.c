/* Expected: main returns 0 (built for the EEPROM: boot loader, eeprom_*) */
/* Returns the number of the first failed check. Uses printf so that the
   boot loader copies several pages; eeprom.screen is what it prints. */

#include <hbc2.h>
#include <stdio.h>
#include <string.h>

static const char text[] = "Copied from the EEPROM at power-on";
static unsigned char buffer[300];
static unsigned char back[300];
static int counter = 1234;      /* .data: copied by the boot loader */

int boot(void)
{
    unsigned char *code = (unsigned char *)boot;
    unsigned i;
    if (eeprom_port == 0 || monitor_port == 0 || rtc_port == 0 || keyboard_port == 0)
        return 1;
    if (counter != 1234)
        return 2;
    /* the program is in the EEPROM at its RAM addresses */
    for (i = 0; i < sizeof text; i++)
        if (eeprom_read((unsigned)text + i) != (unsigned char)text[i])
            return 3;
    for (i = 0; i < 16; i++)
        if (eeprom_read((unsigned)code + i) != code[i])
            return 4;
    return 0;
}

int bytes(void)
{
    if (eeprom_read(EEPROM_FREE) != 0)              /* the rest is blank */
        return 10;
    if (eeprom_write(EEPROM_FREE, 0xA5) || eeprom_read(EEPROM_FREE) != 0xA5)
        return 11;
    if (eeprom_write(0xFFFFFL, 0x5A) || eeprom_read(0xFFFFFL) != 0x5A)
        return 12;                                  /* the last byte */
    if (eeprom_read(EEPROM_SIZE) != -1 || eeprom_write(EEPROM_SIZE, 1) != 1)
        return 13;                                  /* past the end */
    if (eeprom_read(0x1000000L) != -1 || eeprom_read(0xFF000000L) != -1)
        return 14;
    return 0;
}

int blocks(void)
{
    unsigned i;
    for (i = 0; i < sizeof buffer; i++)
        buffer[i] = i * 7 + 3;
    /* across a 64 KiB boundary: ADDR_0 changes */
    if (eeprom_save(0x1FF00L, buffer, sizeof buffer))
        return 20;
    if (eeprom_read(0x1FF00L) != buffer[0] || eeprom_read(0x20000L) != buffer[256])
        return 21;
    if (eeprom_load(back, 0x1FF00L, sizeof back) || memcmp(buffer, back, sizeof back))
        return 22;
    /* across 256-byte pages, from an odd address */
    memset(back, 0, sizeof back);
    if (eeprom_save(0x300F1L, buffer, 200) || eeprom_load(back, 0x300F1L, 200))
        return 23;
    if (memcmp(buffer, back, 200) || back[200] != 0)
        return 24;
    /* up to the last byte, and one past it */
    if (eeprom_load(back, 0xFFFF0L, 16) || back[15] != 0x5A)
        return 25;
    if (eeprom_load(back, 0xFFFF0L, 17) != 1 || eeprom_save(0xFFFF1L, buffer, 16) != 1)
        return 26;
    if (eeprom_load(back, EEPROM_SIZE, 0) != 1 || eeprom_load(back, 0xFFFFFL, 0))
        return 27;
    return 0;
}

void finish(int r)
{
    if (r)
        exit(r);
}

int main(void)
{
    finish(boot());
    finish(bytes());
    finish(blocks());
    printf("%s\n%d\n", text, counter);
    return 0;
}
