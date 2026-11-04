# To check on the breadboard

What the paper analysis can't prove; measure it on the breadboard with a scope. Add each item with the section that needs it and why.

| # | Section | What to measure | Pass | Why |
|---|---|---|---|---|
| B1 | address section (2026-11-05, addr-report.md item 4) | At a `clk` rising edge, how long the control register's outputs (`addr_lat_ld_n`, `addr_pc_ld_n`, `addr_sp_ld_n`) and the 377s' D inputs stay unchanged after the edge, at the 377 pins | /E ≥ 5 ns, D ≥ 3 ns (74HCT377 hold, TI SCHS184C) | TI gives no minimum CP → Q for the control register's 574 (typ 15 ns), so the 377's hold times can't be proven on paper (addr-decisions.md A4). |
