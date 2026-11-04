// fenv.h - HBC-2 C library: floating-point exceptions (asm/fmath.has)
//
// Every float operation and math.h function raises these flags; they stay
// raised until the program clears them (they are all clear when it
// starts). Nothing traps. Rounding is always to nearest even.
// Constants the compiler folds (1.0 / 0.0 in a static initializer, for
// instance) raise nothing.

#ifndef __FENV_H
#define __FENV_H

#define FE_INVALID    0x01      // no meaningful result: NaN (0/0, sqrt(-1)...)
#define FE_DIVBYZERO  0x02      // an exact infinity from finite operands (1/0, log(0))
#define FE_OVERFLOW   0x04      // too large: infinity
#define FE_UNDERFLOW  0x08      // tiny (subnormal or 0) and rounded
#define FE_INEXACT    0x10      // the result was rounded
#define FE_ALL_EXCEPT 0x1F

#define FE_TONEAREST  0         // the only rounding mode

typedef unsigned char fexcept_t;
typedef unsigned char fenv_t;   // the flags

extern const fenv_t __fe_dfl_env;
#define FE_DFL_ENV    (&__fe_dfl_env)   // no flag raised

int            feclearexcept(int excepts);
int            feraiseexcept(int excepts);
int            fetestexcept(int excepts);   // the ones of excepts that are raised
int            fegetexceptflag(fexcept_t *flagp, int excepts);
int            fesetexceptflag(const fexcept_t *flagp, int excepts);

int            fegetround(void);            // FE_TONEAREST
int            fesetround(int round);       // 0 for FE_TONEAREST, else 1 (refused)

int            fegetenv(fenv_t *envp);
int            fesetenv(const fenv_t *envp);
int            feholdexcept(fenv_t *envp);  // saves the flags and clears them
int            feupdateenv(const fenv_t *envp); // *envp, plus the flags raised since

#endif
