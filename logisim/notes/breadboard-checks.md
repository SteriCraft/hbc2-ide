# To check on the breadboard

What the paper analysis can't prove; measure it on the breadboard with a scope. Add each item with the section that needs it and why.

| # | Section | What to measure | Pass | Why |
|---|---|---|---|---|
| B1 | address section (2026-11-05, addr-report.md item 4) | At a `clk` rising edge, how long the control register's outputs (`addr_lat_ld_n`, `addr_pc_ld_n`, `addr_sp_ld_n`) and the 377s' D inputs stay unchanged after the edge, at the 377 pins | /E ≥ 5 ns, D ≥ 3 ns (74HCT377 hold, TI SCHS184C) | TI gives no minimum CP → Q for the control register's 574 (typ 15 ns), so the 377's hold times can't be proven on paper (addr-decisions.md A4). |
| B2 | memory (2026-11-05, Gianni's answer 5, ir-mem-decisions.md M1) | At the RAM pins (AS6C1008 WE#, A0–A15): the WE# low pulse width, and the time from the last address change (after the `clk` edge) to WE# falling | pulse ≥ 45 ns (tWP; 174 ns on paper at 25 °C), address stable ≥ 0 ns before WE# falls (tAS; 25 ns on paper at 25 °C, −2 at 85 °C with FF1 at max and FF2 at 0) and until WE# rises (tWR 0) | The pulse and the setup depend on the spread between the two flip-flops of the clock generator's 74HCT74 and on the minimum delays, which TI doesn't specify. |
