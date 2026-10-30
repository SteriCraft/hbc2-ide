"""Generates ops/<type>_<group>_<g|l>.c: every operator on one C type, with the expected
results computed here with HBC-2 C rules (int 16 bits, char unsigned,
long 32 bits, usual arithmetic conversions). Each program returns 0, or the
number of the first check that failed.

Each group of checks runs twice: on globals (_g) and on locals (_l), because
the compiler reads and writes them differently (64 KB of memory: one
program per group). They use variables
and constants as operands, conditions (alone, negated, in && / || chains),
loops, every constant shift count, multiplies by constants, powers of two,
and assignment operators on variables, through pointers and struct fields.

    python gen_ops.py      (writes the files into ops/ next to this script)
"""
import os
import random

TYPES = {
    # name: (bits, signed, literal suffix)
    'char': (8, False, ''),
    'signed char': (8, True, ''),
    'int': (16, True, ''),
    'unsigned': (16, False, 'u'),
    'long': (32, True, 'L'),
    'unsigned long': (32, False, 'UL'),
}

COMPARISONS = ['<', '>', '<=', '>=', '==', '!=']


def wrap(v, bits, signed):
    v &= (1 << bits) - 1
    if signed and v >= 1 << (bits - 1):
        v -= 1 << bits
    return v


def promoted(name):
    """Type after the integer promotions: char → int."""
    return 'int' if TYPES[name][0] < 16 else name


def literal(v, name):
    bits, signed, suffix = TYPES[name]
    v = wrap(v, bits, signed)
    if v == -(1 << (bits - 1)):
        return f'({v + 1}{suffix} - 1)'      # -32768 as a constant can get the wrong type
    if v < 0:
        return f'({v}{suffix})'
    return f'0x{v:X}{suffix}' if name != 'char' and name != 'signed char' else str(v)


def c_div(a, b):
    q = abs(a) // abs(b)
    return q if (a < 0) == (b < 0) else -q


def c_rem(a, b):
    return a - c_div(a, b) * b


def binop(op, a, b, name):
    """a op b on values of type name, in the promoted type. None if undefined."""
    t = promoted(name)
    bits, signed, _ = TYPES[t]
    a, b = wrap(a, bits, signed), wrap(b, bits, signed)
    if op in '/%' and (b == 0 or (signed and a == -(1 << (bits - 1)) and b == -1)):
        return None
    if op in ('<<', '>>') and not 0 <= b < bits:
        return None
    r = {
        '+': lambda: a + b, '-': lambda: a - b, '*': lambda: a * b,
        '/': lambda: c_div(a, b), '%': lambda: c_rem(a, b),
        '&': lambda: a & b, '|': lambda: a | b, '^': lambda: a ^ b,
        '<<': lambda: a << b, '>>': lambda: a >> b,       # Python >> is arithmetic
        '<': lambda: int(a < b), '>': lambda: int(a > b),
        '<=': lambda: int(a <= b), '>=': lambda: int(a >= b),
        '==': lambda: int(a == b), '!=': lambda: int(a != b),
    }[op]()
    if op in COMPARISONS:
        return r, 'int'
    return wrap(r, bits, signed), t


def values(name, rng):
    bits, signed, _ = TYPES[name]
    special = [0, 1, 2, 3, 7, (1 << (bits - 1)) - 1, 1 << (bits - 1), (1 << bits) - 1, 100]
    out = []
    for _ in range(3):
        if rng.random() < 0.4:
            out.append(rng.choice(special))
        elif rng.random() < 0.5:
            out.append(rng.randrange(1 << bits))
        else:
            out.append(rng.randrange(1, 50))
    return [wrap(v, bits, signed) for v in out]


class Program:
    def __init__(self, name, rng):
        self.name = name
        self.rng = rng
        self.bits, self.signed, _ = TYPES[name]
        self.p = promoted(name)
        self.pv = self.result_var(self.p)
        self.lines = []
        self.n = 0

    @staticmethod
    def result_var(t):
        return 'r_' + t.replace(' ', '_')

    def lit(self, v):
        return literal(v, self.name)

    def pick(self):
        return self.rng.choice(values(self.name, self.rng))

    def check(self, setup, expr_var, expected, rtype):
        self.n += 1
        self.lines.append(f'    {setup}')
        self.lines.append(f'    if ({expr_var} != {literal(expected, rtype)})')
        self.lines.append(f'        return {self.n};')

    def new_value(self, op, a, b):
        """Value of the variable after 'v op= b'."""
        got = binop(op, a, b, self.name)
        return None if got is None else wrap(got[0], self.bits, self.signed)

    def operators(self):
        """Each operator on two variables, a variable and a constant, a constant and a variable."""
        lit, rng = self.lit, self.rng
        for op in ['+', '-', '*', '/', '%', '&', '|', '^', '<<', '>>'] + COMPARISONS:
            for form in ('vv', 'vv', 'vv', 'vk', 'vk', 'kv'):
                a, b = self.pick(), self.pick()
                if op in ('<<', '>>'):
                    if form == 'kv':
                        continue
                    b = rng.randrange(TYPES[self.p][0])
                if form == 'kv':
                    a, b = b, a
                got = binop(op, a, b, self.name)
                if got is None:
                    continue
                r, t = got
                rv = self.result_var(t)
                if form == 'vv':
                    self.check(f'a = {lit(a)}; b = {lit(b)}; {rv} = a {op} b;', rv, r, t)
                elif form == 'vk':
                    k = str(b) if op in ('<<', '>>') else lit(b)
                    self.check(f'a = {lit(a)}; {rv} = a {op} {k};', rv, r, t)
                else:
                    self.check(f'b = {lit(b)}; {rv} = {lit(a)} {op} b;', rv, r, t)

    def conditions(self):
        """Comparisons as conditions: alone, negated, in && / || chains."""
        lit, rng, name = self.lit, self.rng, self.name
        for op in COMPARISONS:
            for _ in range(4):
                a, b, k = self.pick(), self.pick(), self.pick()
                if rng.random() < 0.3:
                    k = a
                r1 = binop(op, a, b, name)[0]
                r2 = binop(op, a, k, name)[0]
                self.check(f'a = {lit(a)}; b = {lit(b)}; if (a {op} b) r_int = 1; else r_int = 0;',
                           'r_int', r1, 'int')
                self.check(f'a = {lit(a)}; if (a {op} {lit(k)}) r_int = 1; else r_int = 0;',
                           'r_int', r2, 'int')
                self.check(f'a = {lit(a)}; if (!(a {op} {lit(k)})) r_int = 1; else r_int = 0;',
                           'r_int', 1 - r2, 'int')
        for _ in range(12):
            a, b, k1, k2 = self.pick(), self.pick(), self.pick(), self.pick()
            o1, o2, o3 = rng.choice(COMPARISONS), rng.choice(COMPARISONS), rng.choice(COMPARISONS)
            r1 = binop(o1, a, b, name)[0]
            r2 = binop(o2, a, k1, name)[0]
            r3 = binop(o3, b, k2, name)[0]
            ab = f'a = {lit(a)}; b = {lit(b)};'
            self.check(f'{ab} if (a {o1} b && a {o2} {lit(k1)}) r_int = 1; else r_int = 0;',
                       'r_int', r1 & r2, 'int')
            self.check(f'{ab} if (a {o1} b || a {o2} {lit(k1)}) r_int = 1; else r_int = 0;',
                       'r_int', r1 | r2, 'int')
            self.check(f'{ab} if (a {o1} b || (a {o2} {lit(k1)} && !(b {o3} {lit(k2)}))) '
                       f'r_int = 1; else r_int = 0;', 'r_int', r1 | (r2 & (1 - r3)), 'int')
            self.check(f'{ab} r_int = (a {o1} b) && b {o3} {lit(k2)};', 'r_int', r1 & r3, 'int')
            self.check(f'{ab} r_int = 0; if (a) r_int = 1; if (!b) r_int += 2;',
                       'r_int', (a != 0) + 2 * (b == 0), 'int')

    def loops(self):
        lit, bits, signed = self.lit, self.bits, self.signed
        top = (1 << (bits - 1)) - 1 if signed else (1 << bits) - 1
        for _ in range(3):
            lo = min(self.pick(), top - 21)             # no wrap around in the loops
            hi = lo + self.rng.randrange(0, 20)
            self.check(f'r_int = 0; for (v = {lit(lo)}; v < {lit(hi)}; v++) r_int++;',
                       'r_int', hi - lo, 'int')
            self.check(f'r_int = 0; v = {lit(hi)}; while (v > {lit(lo)}) {{ v--; r_int += 2; }}',
                       'r_int', 2 * (hi - lo), 'int')
            self.check(f'r_int = 0; v = {lit(lo)}; '
                       f'do {{ r_int++; }} while (++v <= {lit(hi)} && v != {lit(top)});',
                       'r_int', hi - lo + 1, 'int')

    def constants(self):
        """Every constant shift count, multiplies by constants, / and % by powers of two."""
        lit, p, pv, name = self.lit, self.p, self.pv, self.name
        tb, ts, _ = TYPES[p]
        for k in range(tb):
            a = self.pick()
            self.check(f'a = {lit(a)}; {pv} = a << {k};', pv, binop('<<', a, k, name)[0], p)
            self.check(f'a = {lit(a)}; {pv} = a >> {k};', pv, binop('>>', a, k, name)[0], p)
        for k in list(range(0, 26)) + [31, 32, 33, 40, 64, 100, 255, 256, 1000]:
            a = self.pick()
            r, t = binop('*', a, k, name)
            self.check(f'a = {lit(a)}; {pv} = a * {k};', pv, r, t)
        for m in range(0, tb, 3 if tb > 16 else 1):
            a, k = self.pick(), 1 << m
            for op in '/%':
                got = binop(op, a, k, name)
                if got is not None:
                    self.check(f'a = {lit(a)}; {pv} = a {op} {literal(k, p)};', pv, got[0], got[1])

    def assignments(self):
        """Assignment operators on the variable, through a pointer, on a struct field."""
        lit, name, bits, signed = self.lit, self.name, self.bits, self.signed
        for op in ['+=', '-=', '&=', '|=', '^=']:
            for _ in range(2):
                a, k, b = self.pick(), self.pick(), self.pick()
                new = self.new_value(op[:-1], a, k)
                self.check(f'v = {lit(a)}; v {op} {lit(k)};', 'v', new, name)
                self.check(f'v = {lit(a)}; a = (v {op} {lit(k)}); b = v;', 'a', new, name)
                self.check(f'v = {lit(a)}; p = &v; *p {op} {lit(k)};', 'v', new, name)
                self.check(f'st.f = {lit(a)}; ps = &st; a = (ps->f {op} {lit(k)});', 'a', new, name)
                self.check(f'st.f = {lit(a)}; ps = &st; ps->f {op} {lit(k)};', 'st.f', new, name)
                new = self.new_value(op[:-1], a, b)
                self.check(f'v = {lit(a)}; p = &v; b = {lit(b)}; *p {op} b;', 'v', new, name)
        for _ in range(2):
            a, k = self.pick(), self.pick()
            up, down = wrap(a + 1, bits, signed), wrap(a - 1, bits, signed)
            self.check(f'st.f = {lit(a)}; ps = &st; ps->f++;', 'st.f', up, name)
            self.check(f'st.f = {lit(a)}; ps = &st; a = ps->f--;', 'a', a, name)
            self.check(f'v = {lit(a)}; p = &v; a = ++*p;', 'a', up, name)
            self.check(f'v = {lit(a)}; p = &v; (*p)--;', 'v', down, name)
            self.check(f'v = {lit(a)}; p = &v; a = (*p)++; b = v;', 'b', up, name)
            self.check(f'p = &v; *p = {lit(k)}; ps = &st; ps->f = v; a = ps->f;', 'a', k, name)
        for op in ['+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>=']:
            for _ in range(2):
                a, b = self.pick(), self.pick()
                if op in ('<<=', '>>='):
                    b = self.rng.randrange(TYPES[self.p][0])
                new = self.new_value(op[:-1], a, b)
                if new is None:
                    continue
                self.check(f'v = {lit(a)}; b = {lit(b)}; v {op} b;', 'v', new, name)
                self.check(f'v = {lit(a)}; b = {lit(b)}; a = (v {op} b);', 'a', new, name)
        for _ in range(2):
            a = self.pick()
            up, down = wrap(a + 1, bits, signed), wrap(a - 1, bits, signed)
            self.check(f'v = {lit(a)}; a = v++;', 'a', a, name)
            self.check(f'v = {lit(a)}; v++;', 'v', up, name)
            self.check(f'v = {lit(a)}; a = v--;', 'a', a, name)
            self.check(f'v = {lit(a)}; v--;', 'v', down, name)
            self.check(f'v = {lit(a)}; a = ++v;', 'a', up, name)
            self.check(f'v = {lit(a)}; a = --v;', 'a', down, name)

    def unary_and_switch(self):
        lit, p, pv, name, bits, signed = self.lit, self.p, self.pv, self.name, self.bits, self.signed
        tb, ts, _ = TYPES[p]
        for _ in range(2):
            a = self.pick()
            self.check(f'a = {lit(a)}; {pv} = -a;', pv, wrap(-a, tb, ts), p)
            self.check(f'a = {lit(a)}; {pv} = ~a;', pv, wrap(~a, tb, ts), p)
            self.check(f'a = {lit(a)}; r_int = !a;', 'r_int', int(a == 0), 'int')
        cases = sorted(set(values(name, self.rng) + values(name, self.rng)))
        body = ' '.join(f'case {lit(c)}: r_int = {k + 1}; break;' for k, c in enumerate(cases))
        for probe in cases[:3] + [wrap(cases[-1] + 1, bits, signed)]:
            want = cases.index(probe) + 1 if probe in cases else 99
            self.check(f'a = {lit(probe)}; switch (a) {{ {body} default: r_int = 99; }}',
                       'r_int', want, 'int')

    GROUPS = {
        'ops': ['operators'],
        'cond': ['conditions', 'loops'],
        'const': ['constants'],
        'assign': ['assignments', 'unary_and_switch'],
    }

    def generate(self, group, local):
        """One program: the checks of one group, on globals or on locals."""
        name, p = self.name, self.p
        decls = [f'{name} a, b, v;', 'int r_int;']
        if p != 'int':
            decls.append(f'{p} {self.pv};')
        decls += [f'struct rec {{ char pad; {name} f; }} st;', f'{name} *p;', 'struct rec *ps;']
        where = 'locals' if local else 'globals'
        self.lines = [f'/* Expected: main returns 0. Generated by gen_ops.py: {group} checks '
                      f'on {name}, {where}. */']
        if not local:
            self.lines += decls
        self.lines.append('int main(void)\n{')
        if local:
            self.lines += ['    ' + d for d in decls]
        self.n = 0
        for part in self.GROUPS[group]:
            getattr(self, part)()
        self.lines.append('    return 0;\n}')
        return '\n'.join(self.lines) + '\n'


def main():
    here = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'ops')
    os.makedirs(here, exist_ok=True)
    rng = random.Random(2026_10_30)
    for name in TYPES:
        for group in Program.GROUPS:
            for local in (False, True):
                file = f"{name.replace(' ', '_')}_{group}_{'l' if local else 'g'}.c"
                with open(os.path.join(here, file), 'w', newline='\n') as f:
                    f.write(Program(name, rng).generate(group, local))
                print(file)


if __name__ == '__main__':
    main()
