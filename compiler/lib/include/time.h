// time.h - HBC-2 C library: date and time from the I/O processor's clock,
// clock() from the native display's frames (code in
// compiler/lib/asm/time.has).
// time_t counts the seconds since 1970-01-01 00:00:00, unsigned 32 bits:
// from 1970 to 2106-02-07 06:28:15. The RTC keeps the host computer's
// local time and knows no time zone: gmtime is the same as localtime.
// No difftime (it returns a double): subtract two time_t instead.

#ifndef __TIME_H
#define __TIME_H

#ifndef NULL
#define NULL ((void *)0)
#endif

#ifndef __SIZE_T
#define __SIZE_T
typedef unsigned size_t;
#endif

typedef unsigned long time_t;
typedef long clock_t;

#define CLOCKS_PER_SEC 60

struct tm {
    int tm_sec;     // 0-59 (60 allowed in mktime)
    int tm_min;     // 0-59
    int tm_hour;    // 0-23
    int tm_mday;    // 1-31
    int tm_mon;     // 0-11: January is 0
    int tm_year;    // years since 1900: 2026 is 126
    int tm_wday;    // 0-6: Sunday is 0
    int tm_yday;    // 0-365: January 1 is 0
    int tm_isdst;   // always 0
};

// The I/O processor's date and time, also put at t when t is not NULL.
// (time_t)-1 when there is no clock, or when it is set before 1970.
time_t time(time_t *t);

// The native display's frames (CLOCKS_PER_SEC = 60) since the first call
// to clock() or delay() (hbc2.h): 0 at first. (clock_t)-1 without a
// native display.
clock_t clock(void);

// The time of *tm. Its fields may be out of range first (tm_mday 0 is
// the last day of the month before, tm_min 90 is 1 hour 30, ...): mktime
// puts them in range and sets tm_wday, tm_yday and tm_isdst. (time_t)-1
// when the date is before 1970 or past 2106-02-07 06:28:15.
time_t mktime(struct tm *tm);

// The date and time of *t. The result is one struct that the next call
// to localtime or gmtime writes again.
struct tm *localtime(const time_t *t);
struct tm *gmtime(const time_t *t);

// Like "Sun Sep 16 01:03:52 1973\n": 25 characters and the 0, in one
// buffer that the next call to asctime or ctime writes again.
char *asctime(const struct tm *tm);
char *ctime(const time_t *t);       // asctime(localtime(t))

// Writes fmt at s, with the fields of *tm in place of:
//   %a Sun  %A Sunday  %b Jan  %B January  %d 01-31  %e  1-31 (C99)
//   %H 00-23  %I 01-12  %p AM or PM  %M 00-59  %S 00-60
//   %j 001-366 (day of the year)  %m 01-12  %y 00-99  %Y 2026
//   %w 0-6 (Sunday is 0)  %U 00-53 (week, from the first Sunday)
//   %W 00-53 (week, from the first Monday)
//   %c Sun Jan  1 00:00:00 2000  %x 01/31/00  %X 23:59:59
//   %Z nothing (no time zone)  %% a %
// At most max characters, the final 0 included. Returns the number of
// characters written (the 0 left out), or 0 when they did not all fit.
size_t strftime(char *s, size_t max, const char *fmt, const struct tm *tm);

#endif
