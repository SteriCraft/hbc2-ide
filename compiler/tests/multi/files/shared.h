/* shared.h - what main.c, counter.c and add3.has share */

extern int total;                       /* main.c */
extern int table[3];                    /* counter.c */

int counter_next(void);
int counter_triple(int x);
int counter_plus_total(int x);
int counter_apply(int (*f)(int), int x);
int counter_calls(void);
int counter_half(int x);

int add3(int a, int b, int c);          /* add3.has */
