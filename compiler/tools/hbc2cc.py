"""Temporary C compiler driver for the HBC-2, until `hbc2 cc` exists (roadmap step 10).

usage: python hbc2cc.py file.c [-o file.has] [--unit N] [-I dir] [-D NAME[=value]] [--no-copt]

Runs the stages built by compiler/build.cmd (compiler/bin):
cpp -> cc0 -> cc1.hbc2 -> cc2.hbc2 -> copt (rules in compiler/fcc/rules.hbc2;
--no-copt skips it), renames the static names (_name -> __S<unit>_name,
as the Rust driver does; not with --no-copt) and writes the assembly
(file.has by default). #include "x.h" looks next to the C file first; the C
library headers (compiler/lib/include) are on the include path. One file
only: a program of several files is built by the Rust driver
(hbc2_cc::build_program).
Temporary files (.%, .@, .#, .sym, .^) go next to the output and are removed.
"""
import argparse
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(HERE, '..', 'bin')
RULES = os.path.join(HERE, '..', 'fcc', 'rules.hbc2')
LIB_INCLUDE = os.path.join(HERE, '..', 'lib', 'include')


def tool(name):
    return os.path.join(BIN, name + '.exe')


def run(args, src, dst):
    # cc1 reads back its own output: the output file is opened read + write
    with open(src, 'rb') as i, open(dst, 'w+b') as o:
        r = subprocess.run(args, stdin=i, stdout=o)
    if r.returncode:
        sys.exit(f'{os.path.basename(args[0])} failed')


def static_names(asm):
    """Names defined without a '; export' line: the C static names."""
    exported, defined = set(), set()
    for line in asm.split('\n'):
        if line.startswith('; export '):
            exported.add(line[9:].strip())
            continue
        if line.startswith('.data ') or line.startswith('.bss '):
            words = line.split()
            name = words[1] if len(words) > 1 else ''
        elif line.endswith(':'):
            name = line[:-1]
        else:
            continue
        if name.startswith('_') and not name.startswith('__'):
            defined.add(name)
    return defined - exported


def rename_statics(asm, unit):
    """_name -> __S<unit>_name for the static names, not in quotes or comments."""
    statics = static_names(asm)
    out, i, quote, comment = [], 0, None, False
    while i < len(asm):
        c = asm[i]
        if comment:
            comment = c != '\n'
        elif quote:
            if c == '\\':
                out.append(asm[i:i + 2])
                i += 2
                continue
            if c == quote:
                quote = None
        elif c in '"\'':
            quote = c
        elif c == ';':
            comment = True
        elif c.isalnum() or c == '_':
            j = i
            while j < len(asm) and (asm[j].isalnum() or asm[j] == '_'):
                j += 1
            word = asm[i:j]
            out.append(f'__S{unit}{word}' if word in statics else word)
            i = j
            continue
        out.append(c)
        i += 1
    return ''.join(out)


def main():
    p = argparse.ArgumentParser()
    p.add_argument('source')
    p.add_argument('-o', dest='output')
    p.add_argument('--unit', type=int, default=1)
    p.add_argument('-I', dest='includes', action='append', default=[])
    p.add_argument('-D', dest='defines', action='append', default=[])
    p.add_argument('--no-copt', action='store_true')
    a = p.parse_args()

    out = a.output or os.path.splitext(a.source)[0] + '.has'
    base = os.path.splitext(out)[0]
    tmp = {k: base + k for k in ('.%', '.@', '.#', '.sym', '.^')}
    try:
        here = os.path.dirname(os.path.abspath(a.source))
        cpp = [tool('cpp'), '-E', '-D__hbc2__'] \
            + ['-I' + i for i in [here] + a.includes + [LIB_INCLUDE]] \
            + ['-D' + d for d in a.defines] + [a.source]
        with open(tmp['.%'], 'wb') as o:
            if subprocess.run(cpp, stdout=o).returncode:
                sys.exit('cpp failed')
        run([tool('cc0'), tmp['.sym']], tmp['.%'], tmp['.@'])
        run([tool('cc1.hbc2'), '0', '0'], tmp['.@'], tmp['.#'])
        cc2_out = out if a.no_copt else tmp['.^']
        run([tool('cc2.hbc2'), tmp['.sym'], '0', '0', '0', str(a.unit)], tmp['.#'], cc2_out)
        if not a.no_copt:
            run([tool('copt'), RULES], tmp['.^'], out)
            with open(out, encoding='latin-1', newline='') as f:
                asm = f.read()
            with open(out, 'w', encoding='latin-1', newline='') as f:
                f.write(rename_statics(asm, a.unit))
    finally:
        for f in tmp.values():
            if os.path.exists(f):
                os.remove(f)


if __name__ == '__main__':
    main()
