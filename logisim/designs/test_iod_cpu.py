#!/usr/bin/env python3
"""CPU + mailbox bench -> circuits/iod/test_iod_cpu.circ (and test_iod_cpu_eager.circ)

The whole CPU (cpu.circ `main`) with the real I/O card (circuits/iod/iod.circ)
on its connector, and a test MCU on the card's ESP32 side, for
tools/cputest.py (`--bench iod`, `--bench iod_eager`). Like
circuits/cpu/test_cpu.circ, but the stub's port latches and interrupt source
are replaced by the mailbox.

The test MCU is a state machine clocked by `osc` (125 ns a tick, asynchronous
to the CPU's steps as far as the card can tell): a state register and a ROM
(FSM below), and a small data path (IN, B, V, OUT registers, three ROMs).
Its "firmware":

    idle: when TX is full, read it (MBX_SEL low, QH sampled, 7 SCK pulses,
          MBX_SEL high: TX clears) -> B; V = B + 1
    item: shift 16 bits out (MOSI, SCK): for an event (B bit 7 set) V then
          port(V) = 0xF0 | (V & 0x0F); for a reply, V twice (the RX 595 keeps
          the last 8). Then wait until RX full (EVT pending) reads 0, then
          8 ticks (the firmware rule, I15: >= 1 us), then pulse RCLK_RX
          (RCLK_EVT).
    B bit 6 set: a second item with V + 1, right after the first.
    rst_n low (the CPU's reset): back to idle, the item dropped.

The "eager" build (test_iod_cpu_eager.circ) waits 0 ticks: it breaks the
rule, and a second reply or event written while the CPU's clear is still
active is lost (I15). The test programs count what arrives.

The "reset" builds (test_iod_cpu_reset_<v>.circ, cputest --bench iod_reset_<v>)
are the rule-keeping MCU that pulls the CPU's reset once, in the middle of a
transfer (RESETS below): its ROM gets a `fired` input and a `rst_req` output,
and rst_req clears the reset RC (the 74161 and the 7474), so rc is low for 16
osc periods as at power-up. The CPU boots again and runs the program from the
start; the MCU drops its item on rst_n. cputest follows the second boot.

The output pins are test_cpu's (main's connector, buses, test points, int,
vec), so cputest's observer reads both benches the same way; the reset
builds add the MCU's state and flags (OBS_RESET).

    python test_iod_cpu.py      # write all, check
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
sys.path.insert(0, str(ROOT / "designs"))
from circgen import Project, pin_tunnels  # noqa: E402
import cpu as cpu_design  # noqa: E402  (main's pins)

OUTS = {False: ROOT / "circuits" / "iod" / "test_iod_cpu.circ",
        True: ROOT / "circuits" / "iod" / "test_iod_cpu_eager.circ"}
OBS = cpu_design.OUTPUTS + [("int", 1), ("vec", 1)]
OBS_RESET = OBS + [("st", 7), ("fired", 1), ("m_tx", 1), ("m_rx", 1), ("m_evt", 1)]
RULE_TICKS = 8                                   # 8 x 125 ns = 1 us
# reset variants: the MCU state where it pulls the reset, and (b7, b6) of the byte it serves (None: any).
# tx:  in the middle of reading TX (MBX_SEL low, 4 bits shifted), the first byte (0x10);
# rx:  phase 2 (two replies): reply 1 waiting in RX, reply 2 half shifted into the RX 595;
# evt: phase 4 (two events, interrupts off): event 1 pending (INT high), event 2 half shifted.
RESETS = {"tx": ("H3", None), "rx": ("I2_S8H", (0, 1)), "evt": ("I2_S8H", (1, 1))}
for _v in RESETS:
    OUTS[("reset", _v)] = ROOT / "circuits" / "iod" / f"test_iod_cpu_reset_{_v}.circ"


# ==== the test MCU's state machine ======================================================================
# ROM address: state (7 bits) | tx_full << 7 | rx_full << 8 | evt_pend << 9 | b7 << 10 | b6 << 11 | rst_n << 12
#              (reset builds: | fired << 13)
# ROM data:    next (7 bits) | sck << 7 | mbx_sel << 8 | rclk_rx << 9 | rclk_evt << 10 | in_en << 11 | b_en << 12
#              | v_en << 13 | v_sel_b << 14 | out_en << 15 | out_load << 16 (reset builds: | rst_req << 17)
OUT_BITS = {"sck": 7, "mbx_sel": 8, "rclk_rx": 9, "rclk_evt": 10, "in_en": 11, "b_en": 12, "v_en": 13,
            "v_sel_b": 14, "out_en": 15, "out_load": 16, "rst_req": 17}


def fsm(wait: int, reset: str | None = None) -> list[int]:
    names = ["IDLE", "R0", "R1"]
    for b in range(6, -1, -1):
        names += [f"H{b}", f"L{b}"]
    names += ["RDONE", "DEC"]
    for i in (1, 2):
        names.append(f"I{i}_LOAD")
        for k in range(16):
            names += [f"I{i}_S{k}L", f"I{i}_S{k}H"]
        names.append(f"I{i}_WF")
        names += [f"I{i}_W{k}" for k in range(wait)]
        names += [f"I{i}_P", f"I{i}_PE"]
    idx = {n: k for k, n in enumerate(names)}
    assert len(names) <= 128, len(names)

    def word(nxt, **o):
        v = idx[nxt]
        o.setdefault("mbx_sel", 1)
        for k, b in o.items():
            v |= int(b) << OUT_BITS[k]
        return v

    rom = [0] * (1 << (14 if reset else 13))
    for addr in range(len(rom)):
        st = addr & 0x7F
        tx, rx, ev, b7, b6, rst, fired = ((addr >> k) & 1 for k in range(7, 14))
        if st >= len(names):
            rom[addr] = word("IDLE")
            continue
        n = names[st]
        if not rst or n == "IDLE":
            rom[addr] = word("R0" if n == "IDLE" and rst and tx else "IDLE")
        elif n == "R0":
            rom[addr] = word("R1", mbx_sel=0)
        elif n == "R1":
            rom[addr] = word("H6", mbx_sel=0, in_en=1)                      # bit 7: QH right after the load
        elif n[0] == "H":
            rom[addr] = word("L" + n[1], mbx_sel=0, sck=1)                   # the 165 shifts
        elif n[0] == "L":
            b = int(n[1])
            rom[addr] = word(f"H{b - 1}" if b else "RDONE", mbx_sel=0, in_en=1)
        elif n == "RDONE":
            rom[addr] = word("DEC", b_en=1)                                  # MBX_SEL rises: TX clears
        elif n == "DEC":
            rom[addr] = word("I1_LOAD", v_en=1, v_sel_b=1)                   # V = B + 1
        else:
            i, part = n[1], n[3:]
            pre = f"I{i}_"
            if part == "LOAD":
                rom[addr] = word(pre + "S0L", out_en=1, out_load=1)
            elif part.startswith("S"):
                k, hl = int(part[1:-1]), part[-1]
                if hl == "L":
                    rom[addr] = word(f"{pre}S{k}H")
                else:                                                         # SCK rises: the 595s take MOSI
                    rom[addr] = word(f"{pre}S{k + 1}L" if k < 15 else pre + "WF", sck=1, out_en=1)
            elif part == "WF":
                busy = ev if b7 else rx
                rom[addr] = word(pre + "WF" if busy else (pre + "W0" if wait else pre + "P"))
            elif part.startswith("W"):
                k = int(part[1:])
                rom[addr] = word(f"{pre}W{k + 1}" if k + 1 < wait else pre + "P")
            elif part == "P":
                rom[addr] = word(pre + "PE", rclk_rx=not b7, rclk_evt=b7)
            elif part == "PE":
                if i == "1" and b6:
                    rom[addr] = word("I2_LOAD", v_en=1, v_sel_b=0)          # V = V + 1
                else:
                    rom[addr] = word("IDLE")
    if reset:                                    # pull the reset once, in the chosen state
        state, bits = RESETS[reset]
        for addr in range(len(rom)):
            st, b7, b6, fired = addr & 0x7F, addr >> 10 & 1, addr >> 11 & 1, addr >> 13 & 1
            # not gated by rst_n: the reset it causes drops m_rst_n at once, which would end the
            # pulse before the osc edge that sets `fired` (it would then fire again at every boot)
            if st == idx[state] and not fired and (bits is None or bits == (b7, b6)):
                rom[addr] |= 1 << OUT_BITS["rst_req"]
    return rom


def build(eager: bool, reset: str | None = None):
    p = Project()
    t = p.circuit(f"test_iod_cpu_reset_{reset}" if reset else "test_iod_cpu_eager" if eager else "test_iod_cpu")
    cpu = p.library(ROOT / "cpu.circ")
    iod = p.library(ROOT / "circuits" / "iod" / "iod.circ")

    def hook(pt, net, width=1, side="L"):
        dx, dy = {"L": (-20, 0), "R": (20, 0), "T": (0, -20), "B": (0, 20)}[side]
        end = (pt[0] + dx, pt[1] + dy)
        t.wire(pt, end)
        if net in ("vcc", "gnd"):
            face = {"L": "west", "R": "east", "T": "north", "B": "south"}[side]
            t.add("#Wiring", "Power" if net == "vcc" else "Ground", end, facing=face,
                  **({"width": width} if width > 1 else {}))
            return
        t.tunnel(end, net, width, facing={"L": "east", "R": "west", "T": "south", "B": "north"}[side])

    def instance(lib, name, loc, nets):
        u = t.sub(lib, name, loc)
        xs = [q.loc[0] for q in u.ports]
        ys = [q.loc[1] for q in u.ports]
        for q in u.ports:
            x, y = q.loc
            side = "L" if x == min(xs) else "R" if x == max(xs) else "T" if y == min(ys) else "B"
            hook((x, y), nets.get(q.name, q.name), q.width, side)
        return u

    def register(loc, d, q, en, width, clk="osc"):
        t.add("#Memory", "Register", loc, width=width, appearance="logisim_evolution")
        x, y = loc
        hook((x, y + 30), d, width)
        hook((x + 60, y + 30), q, width, side="R")
        hook((x, y + 70), clk)
        hook((x, y + 50), en)
        hook((x + 30, y + 90), "gnd", side="B")

    def rom(loc, a, q, table, aw, dw):
        words = " ".join(f"{v:x}" for v in table)
        t.add("#Memory", "ROM", loc, addrWidth=aw, dataWidth=dw, appearance="logisim_evolution",
              contents=f"addr/data: {aw} {dw}\n" + words + "\n")
        hook((loc[0], loc[1] + 10), a, aw)
        hook((loc[0] + 240, loc[1] + 60), q, dw, side="R")

    def buffer(loc, a, en, y, width):
        t.add("#Gates", "Controlled Buffer", loc, width=width)
        hook((loc[0] - 20, loc[1]), a, width)
        hook((loc[0] - 10, loc[1] + 10), en, side="B")
        hook(loc, y, width, side="R")

    def gate_not(loc, a, y):
        t.add("#Gates", "NOT Gate", loc)
        hook((loc[0] - 30, loc[1]), a)
        hook(loc, y, side="R")

    def split(loc, net, width, groups, names, widths):
        """A bus `net` cut into groups (each a list of bits) on tunnels `names`."""
        sp = t.splitter(loc, width, groups, facing="east", appear="right", spacing=2)
        t.tunnel(loc, net, width, facing="east")
        for k, (nm, w) in enumerate(zip(names, widths)):
            e = sp[f"end{k}"]
            t.wire(e, (e[0] + 20, e[1]))
            if nm == "gnd":
                t.add("#Wiring", "Ground", (e[0] + 20, e[1]), facing="east", **({"width": w} if w > 1 else {}))
            else:
                t.tunnel((e[0] + 20, e[1]), nm, w, facing="west")

    def join(loc, net, width, groups, names, widths):
        """Tunnels `names` (groups of bits) gathered into the bus `net`."""
        sp = t.splitter(loc, width, groups, facing="west", appear="left", spacing=2)
        t.wire(loc, (loc[0] + 20, loc[1]))
        t.tunnel((loc[0] + 20, loc[1]), net, width, facing="west")
        for k, (nm, w) in enumerate(zip(names, widths)):
            e = sp[f"end{k}"]
            t.wire(e, (e[0] - 20, e[1]))
            if nm == "gnd":
                t.add("#Wiring", "Ground", (e[0] - 20, e[1]), facing="west", **({"width": w} if w > 1 else {}))
            else:
                t.tunnel((e[0] - 20, e[1]), nm, w, facing="east")

    # ---- the CPU and the card ---------------------------------------------------------------------
    u = t.sub(cpu, "main", (1400, 300))
    for name, w in cpu_design.INPUTS + cpu_design.OUTPUTS:
        x, y = u[name]
        d = -1 if (name, w) in cpu_design.INPUTS else 1
        t.wire((x, y), (x + 20 * d, y))
        t.tunnel((x + 20 * d, y), name, w, facing="east" if d < 0 else "west")
    instance(iod, "iod", (3200, 300), {
        "iod_bus_in": "bus_data", "iod_addr_in": "lo", "iod_ie": "ie", "iod_rw": "rw", "iod_inr": "inr",
        "clk": "clk_5", "iod_bus_out": "iod_bus_data", "iod_addr_out": "iod_bus_addr_lo", "iod_int": "int",
        "iod_sck": "sck", "iod_mosi": "mosi", "iod_rclk_rx": "rclk_rx", "iod_rclk_evt": "rclk_evt",
        "iod_mbx_sel": "mbx_sel", "iod_miso": "miso", "iod_mcu_tx_full": "m_tx", "iod_mcu_rx_full": "m_rx",
        "iod_mcu_evt_pend": "m_evt", "iod_mcu_rst_n": "m_rst_n"})

    # ---- clock and reset (as test_cpu) ------------------------------------------------------------
    t.add("#Wiring", "Clock", (100, 100))
    t.wire((100, 100), (120, 100))
    t.tunnel((120, 100), "osc", facing="west")
    rc_cnt = t.ttl("74161", (300, 400), facing="south", label="U1")
    clr = "rst_req_n" if reset else "vcc"
    pin_tunnels(t, rc_cnt, {"MR/CLR": clr, "CP/CLK": "osc", "D0/A": "gnd", "D1/B": "gnd", "D2/C": "gnd",
                            "D3/D": "gnd", "CE/ENP": "vcc", "PE/LOAD": "vcc", "CET/ENT": "vcc", "TC/RC0": "rc_tc"})
    rc_ff = t.ttl("7474", (300, 700), facing="south", label="U2")
    pin_tunnels(t, rc_ff, {"nCLR1": clr, "D1": "vcc", "CLK1": "rc_tc", "nPRE1": "vcc", "Q1": "rc",
                           "nCLR2": "vcc", "D2": "inr", "CLK2": "clk_5", "nPRE2": "vcc", "Q2": "vec"})
    t.text((260, 300), "reset RC: rc rises after 15 osc periods; vec = INR one step late (observed only)")
    split((300, 1000), "clk", 6, [[5]], ["clk_5"], [1])
    split((300, 1300), "bus_addr", 16, [list(range(8))], ["lo"], [8])

    # ---- the test MCU ------------------------------------------------------------------------------
    X, Y = 400, 1700
    t.text((X, Y - 60), f"test MCU: a state machine on osc ({'eager: breaks the 1 us rule' if eager else 'follows the 1 us rule'})")
    register((X + 600, Y), "st_d", "st", "vcc", 7)
    a_in = ["st", "m_tx", "m_rx", "m_evt", "b_7", "b_6", "m_rst_n"] + (["fired"] if reset else [])
    q_out = ["st_d", "sck", "mbx_sel", "rclk_rx", "rclk_evt", "in_en", "b_en", "v_en", "v_sel_b", "out_en",
             "out_load"] + (["rst_req"] if reset else [])
    aw, dw = 6 + len(a_in), 6 + len(q_out)
    join((X + 300, Y + 300), "fa", aw, [list(range(7))] + [[b] for b in range(7, aw)], a_in, [7] + [1] * (aw - 7))
    rom((X + 600, Y + 300), "fa", "fq", fsm(0 if eager else RULE_TICKS, reset), aw, dw)
    split((X + 1000, Y + 300), "fq", dw, [list(range(7))] + [[b] for b in range(7, dw)], q_out, [7] + [1] * (dw - 7))
    if reset:                                    # fired: set by the one rst_req pulse, never cleared
        t.text((X + 1300, Y - 60), f"reset once: rst_req in state {RESETS[reset][0]} clears the reset RC; fired keeps it from coming back")
        register((X + 1300, Y), "vcc", "fired", "rst_req", 1)
        gate_not((X + 1300, Y + 200), "rst_req", "rst_req_n")
    # IN: shift MISO in (IN = IN << 1 | miso)
    Y2 = Y + 800
    register((X + 600, Y2), "in_d", "inr_q", "in_en", 8)
    split((X + 900, Y2 + 200), "inr_q", 8, [list(range(7)), [7]], ["in_lo7", "in_nc7"], [7, 1])
    join((X + 300, Y2 + 200), "in_d", 8, [[0], list(range(1, 8))], ["miso", "in_lo7"], [1, 7])
    # B: the byte read
    register((X + 600, Y2 + 400), "inr_q", "b", "b_en", 8)
    split((X + 900, Y2 + 600), "b", 8, [list(range(6)), [6], [7]], ["b_nc", "b_6", "b_7"], [6, 1, 1])
    # V: B + 1 or V + 1
    Y3 = Y2 + 900
    rom((X + 200, Y3), "b", "inc_b", [(k + 1) & 0xFF for k in range(256)], 8, 8)
    rom((X + 200, Y3 + 200), "v", "inc_v", [(k + 1) & 0xFF for k in range(256)], 8, 8)
    buffer((X + 700, Y3 + 100), "inc_b", "v_sel_b", "v_d", 8)
    gate_not((X + 700, Y3 + 300), "v_sel_b", "v_sel_v")
    buffer((X + 700, Y3 + 400), "inc_v", "v_sel_v", "v_d", 8)
    register((X + 900, Y3 + 200), "v_d", "v", "v_en", 8)
    # OUT (16 bits): load {V, low} or shift left; MOSI = bit 15
    Y4 = Y3 + 700
    join((X + 300, Y4), "low_a", 9, [list(range(8)), [8]], ["v", "b_7"], [8, 1])
    rom((X + 400, Y4 - 10), "low_a", "low", [(0xF0 | ((k & 0xFF) & 0x0F)) if k >> 8 else k & 0xFF
                                            for k in range(512)], 9, 8)
    join((X + 300, Y4 + 300), "out_ld", 16, [list(range(8)), list(range(8, 16))], ["low", "v"], [8, 8])
    split((X + 300, Y4 + 500), "out_q", 16, [list(range(15)), [15]], ["out_lo15", "mosi"], [15, 1])
    join((X + 700, Y4 + 500), "out_sh", 16, [[0], list(range(1, 16))], ["gnd", "out_lo15"], [1, 15])
    buffer((X + 1000, Y4 + 300), "out_ld", "out_load", "out_d", 16)
    gate_not((X + 1000, Y4 + 450), "out_load", "out_shift")
    buffer((X + 1000, Y4 + 550), "out_sh", "out_shift", "out_d", 16)
    register((X + 1200, Y4 + 300), "out_d", "out_q", "out_en", 16)

    # ---- output pins (test_cpu's) -------------------------------------------------------------------
    t.output((4600, 40), "halt")
    t.wire((4580, 40), (4600, 40))
    t.add("#Wiring", "Ground", (4580, 40), facing="south")
    y = 100
    for name, w in (OBS_RESET if reset else OBS):
        t.output((4600, y), name, w)
        t.wire((4580, y), (4600, y))
        t.tunnel((4580, y), name, w, facing="east")
        y += 40
    return p, t


if __name__ == "__main__":
    bad = 0
    for key, out in OUTS.items():
        p, _ = build(False, key[1]) if isinstance(key, tuple) else build(key)
        bad += p.save(out)
    sys.exit(1 if bad else 0)
