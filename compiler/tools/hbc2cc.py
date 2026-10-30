"""Temporary C compiler driver for the HBC-2, until `hbc2 cc` exists (roadmap step 10).

usage: python hbc2cc.py file.c [-o file.has] [--unit N] [-I dir] [-D NAME[=value]] [--no-copt]

Runs the stages built by compiler/build.cmd (compiler/bin):
cpp -> cc0 -> cc1.hbc2 -> cc2.hbc2 -> copt (rules in compiler/fcc/rules.hbc2;
--no-copt skips it), and writes the assembly (file.has by default).
Temporary files (.%, .@, .#, .sym, .^) go next to the output and are removed.
"""
import argparse
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.path.join(HERE, '..', 'bin')
RULES = os.path.join(HERE, '..', 'fcc', 'rules.hbc2')


def tool(name):
    return os.path.join(BIN, name + '.exe')


def run(args, src, dst):
    # cc1 reads back its own output: the output file is opened read + write
    with open(src, 'rb') as i, open(dst, 'w+b') as o:
        r = subprocess.run(args, stdin=i, stdout=o)
    if r.returncode:
        sys.exit(f'{os.path.basename(args[0])} failed')


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
        cpp = [tool('cpp'), '-E', '-D__hbc2__'] + ['-I' + i for i in a.includes] \
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
    finally:
        for f in tmp.values():
            if os.path.exists(f):
                os.remove(f)


if __name__ == '__main__':
    main()
