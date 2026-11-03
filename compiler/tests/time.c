/* Expected: main returns 0 (time.h: time, clock, mktime, localtime, gmtime, asctime, ctime, strftime) */
/* Returns the number of the first failed check. The RTC of the tests is
   at 2000-01-01 00:00:00 and does not move. */

#include <time.h>
#include <string.h>
#include <stdlib.h>

static struct tm r, t;
static char buf[80];

/* plain C: 1970 to 2106, 2100 is not a leap year */
static int leap(int y)
{
    return y % 4 == 0 && y != 200;
}

static int mlen(int m, int y)
{
    static const unsigned char len[12] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    return len[m] + (m == 1 && leap(y));
}

static void ref(unsigned long s, struct tm *p)
{
    unsigned long days = s / 86400UL, sod = s % 86400UL;
    unsigned d;
    int y = 70, m = 0;
    p->tm_sec = sod % 60;
    p->tm_min = (sod / 60) % 60;
    p->tm_hour = sod / 3600;
    p->tm_wday = (days + 4) % 7;
    d = days;
    while (d >= 365u + leap(y))
        d -= 365 + leap(y++);
    p->tm_year = y;
    p->tm_yday = d;
    while (d >= mlen(m, y))
        d -= mlen(m++, y);
    p->tm_mon = m;
    p->tm_mday = d + 1;
    p->tm_isdst = 0;
}

static int same(const struct tm *a, const struct tm *b)
{
    return memcmp(a, b, sizeof(struct tm)) == 0;
}

/* 1 when s gives the date y-mo-d h:mi:se, day of the week w, of the year yd */
static int at(unsigned long s, int y, int mo, int d, int h, int mi, int se, int w, int yd)
{
    struct tm *p = localtime(&s);
    return p->tm_year == y - 1900 && p->tm_mon == mo - 1 && p->tm_mday == d
        && p->tm_hour == h && p->tm_min == mi && p->tm_sec == se
        && p->tm_wday == w && p->tm_yday == yd && p->tm_isdst == 0;
}

static void set(int y, int mo, int d, int h, int mi, int se)
{
    memset(&t, 0x55, sizeof t);         /* wday, yday, isdst: anything */
    t.tm_year = y - 1900;
    t.tm_mon = mo - 1;
    t.tm_mday = d;
    t.tm_hour = h;
    t.tm_min = mi;
    t.tm_sec = se;
}

/* 1 when t is now y-mo-d h:mi:se */
static int is(int y, int mo, int d, int h, int mi, int se)
{
    return t.tm_year == y - 1900 && t.tm_mon == mo - 1 && t.tm_mday == d
        && t.tm_hour == h && t.tm_min == mi && t.tm_sec == se;
}

static int fmt(const char *f, const char *want)
{
    size_t n = strftime(buf, sizeof buf, f, &t);
    return n == strlen(want) && strcmp(buf, want) == 0;
}

int main(void)
{
    time_t s, s2;
    unsigned i;
    struct tm *p;

    /* time and clock */
    s = 1;
    if (time(NULL) != 946684800UL || time(&s) != 946684800UL || s != 946684800UL)
        return 1;
    if (clock() != (clock_t)-1)
        return 2;

    /* well-known dates */
    if (!at(0, 1970, 1, 1, 0, 0, 0, 4, 0))
        return 3;
    if (!at(946684800UL, 2000, 1, 1, 0, 0, 0, 6, 0))
        return 4;
    if (!at(951782400UL, 2000, 2, 29, 0, 0, 0, 2, 59))
        return 5;
    if (!at(1234567890UL, 2009, 2, 13, 23, 31, 30, 5, 43))
        return 6;
    if (!at(2147483647UL, 2038, 1, 19, 3, 14, 7, 2, 18))
        return 7;
    if (!at(4102444800UL, 2100, 1, 1, 0, 0, 0, 5, 0))
        return 8;
    if (!at(4107542400UL, 2100, 3, 1, 0, 0, 0, 1, 59))      /* no February 29 */
        return 9;
    if (!at(0xFFFFFFFFUL, 2106, 2, 7, 6, 28, 15, 0, 37))
        return 10;
    if (!at(1767225599UL, 2025, 12, 31, 23, 59, 59, 3, 364))
        return 11;
    s = 0;
    if (localtime(&s) != gmtime(&s) || gmtime(&s)->tm_year != 70)
        return 12;

    /* round trips against plain C */
    for (i = 0; i < 300; i++) {
        s = ((unsigned long)rand() << 17) ^ ((unsigned long)rand() << 2) ^ rand();
        if (i < 40)
            s = s % 400000000UL;        /* the first years too */
        ref(s, &r);
        p = localtime(&s);
        if (!same(p, &r))
            return 13;
        memcpy(&t, p, sizeof t);
        t.tm_wday = 9;
        t.tm_yday = -3;
        if (mktime(&t) != s || !same(&t, &r))
            return 14;
    }

    /* mktime puts the fields in range */
    set(2000, 1, 0, 0, 0, 0);
    if (mktime(&t) != 946684800UL - 86400UL || !is(1999, 12, 31, 0, 0, 0) || t.tm_wday != 5 || t.tm_yday != 364)
        return 15;
    set(2000, 14, 1, 0, 0, 0);
    if (mktime(&t) == (time_t)-1 || !is(2001, 2, 1, 0, 0, 0))
        return 16;
    set(2000, 3, 1, 0, 0, -1);
    if (mktime(&t) != 951868800UL - 1 || !is(2000, 2, 29, 23, 59, 59))
        return 17;
    set(2000, 1, 366, 0, 0, 0);
    if (mktime(&t) == (time_t)-1 || !is(2000, 12, 31, 0, 0, 0) || t.tm_yday != 365)
        return 18;
    set(2000, 1, 1, -25, 90, 3725);
    if (mktime(&t) != 946684800UL - 25 * 3600L + 90 * 60L + 3725 || !is(1999, 12, 31, 1, 32, 5))
        return 19;
    set(2001, -13, 1, 0, 0, 0);
    if (mktime(&t) == (time_t)-1 || !is(1999, 11, 1, 0, 0, 0))     /* index -14: 14 months before January 2001 */
        return 20;
    set(2000, 3, -59, 0, 0, 0);         /* 60 days before March 1 (29 in February) */
    if (mktime(&t) != 946684800UL || !is(2000, 1, 1, 0, 0, 0) || t.tm_yday != 0)
        return 21;
    set(2000, 3, -60, 0, 0, 0);         /* across the year */
    if (mktime(&t) != 946684800UL - 86400UL || !is(1999, 12, 31, 0, 0, 0))
        return 21;
    set(2000, 1, 1, 0, 0, 32767);
    if (mktime(&t) != 946684800UL + 32767 || !is(2000, 1, 1, 9, 6, 7))
        return 22;
    set(2000, 1, 1, 0, 0, 60);          /* 60: a leap second becomes the next minute */
    if (mktime(&t) != 946684860UL || !is(2000, 1, 1, 0, 1, 0))
        return 23;

    /* the limits */
    set(1970, 1, 1, 0, 0, 0);
    if (mktime(&t) != 0 || t.tm_wday != 4)
        return 24;
    set(1969, 12, 31, 23, 59, 59);
    if (mktime(&t) != (time_t)-1)
        return 25;
    set(2106, 2, 7, 6, 28, 14);
    if (mktime(&t) != 0xFFFFFFFEUL)
        return 26;
    set(2106, 2, 7, 6, 28, 16);
    if (mktime(&t) != (time_t)-1)
        return 27;
    set(2106, 2, 8, 0, 0, 0);
    if (mktime(&t) != (time_t)-1)
        return 28;
    set(2107, 1, 1, 0, 0, 0);
    if (mktime(&t) != (time_t)-1)
        return 29;
    set(2400, 1, 1, 0, 0, 0);
    if (mktime(&t) != (time_t)-1)
        return 30;

    /* asctime, ctime */
    s = 1234567890UL;
    if (strcmp(asctime(localtime(&s)), "Fri Feb 13 23:31:30 2009\n"))
        return 31;
    s = 0;
    if (strcmp(ctime(&s), "Thu Jan  1 00:00:00 1970\n"))
        return 32;
    s2 = 0xFFFFFFFFUL;
    if (strcmp(ctime(&s2), "Sun Feb  7 06:28:15 2106\n"))
        return 33;

    /* strftime */
    s = 1234567890UL;
    memcpy(&t, localtime(&s), sizeof t);
    if (!fmt("%a %A %b %B %d %e %H %I %j %m %M %p %S", "Fri Friday Feb February 13 13 23 11 044 02 31 PM 30"))
        return 34;
    if (!fmt("%U %w %W|%x|%X|%y %Y|%Z|%%|%c", "06 5 06|02/13/09|23:31:30|09 2009||%|Fri Feb 13 23:31:30 2009"))
        return 35;
    if (!fmt("", "") || !fmt("plain %q text%", "plain q text"))
        return 36;
    s = 0;                              /* 1970-01-01, a Thursday, midnight */
    memcpy(&t, localtime(&s), sizeof t);
    if (!fmt("%I %p %U %W %j %e %y", "12 AM 00 00 001  1 70"))
        return 37;
    t.tm_hour = 12;
    if (!fmt("%I %p", "12 PM"))
        return 38;
    t.tm_wday = 9;
    t.tm_mon = -1;
    if (!fmt("%a %A %b %B", "? ? ? ?"))
        return 39;
    s = 1767225599UL;                   /* 2025-12-31, a Wednesday */
    memcpy(&t, localtime(&s), sizeof t);
    if (!fmt("%j %U %W %A", "365 52 52 Wednesday"))
        return 40;

    /* max: the final 0 included */
    s = 1234567890UL;
    memcpy(&t, localtime(&s), sizeof t);
    memset(buf, 'z', sizeof buf);
    if (strftime(buf, 5, "%Y", &t) != 4 || strcmp(buf, "2009"))
        return 41;
    memset(buf, 'z', sizeof buf);
    if (strftime(buf, 4, "%Y", &t) != 0)
        return 42;
    if (buf[4] != 'z')                  /* nothing past max */
        return 43;
    memset(buf, 'z', sizeof buf);
    if (strftime(buf, 0, "x", &t) != 0 || buf[0] != 'z')
        return 44;
    if (strftime(buf, 1, "", &t) != 0 || buf[0] != 0)
        return 45;
    if (strftime(buf, 1, "x", &t) != 0)
        return 46;
    return 0;
}
