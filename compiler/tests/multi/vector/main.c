/* Expected: main returns 0
   set_vector: the keyboard's key-pressed vector points to an assembly
   handler (keys.has) that calls on_key. main.keys types "ab". */
#include <hbc2.h>

void key_pressed(void);         /* keys.has */
void start_typing(void);        /* keys.has */

volatile unsigned char count;
volatile unsigned char last;

/* Called by the handler, inside the interrupt */
void on_key(unsigned char code)
{
    count++;
    last = code;
}

int main(void)
{
    if (keyboard_port == 0)
        return 1;
    if (set_vector(0, key_pressed) != 1)
        return 2;
    if (set_vector(keyboard_port, key_pressed) != 0)
        return 3;
    start_typing();
    while (count < 2)
        ;
    if (last != 'B')
        return 4;
    return 0;
}
