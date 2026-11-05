(§3.b E3001-delayed · OPEN · in Nets / timing) (overlapping delayed tri-state drivers are a false multidriver error):
* symptom: overlapping delayed tri-state drivers are E3001 (`check_whole_net_multidriver`)
* fix: let `md_nets` resolve delayed drivers (probe at 1 ns)
* oracles: 2 oracles
* status: OPEN.
* record: Its release cells (Q-RELDELAY E15 delayed + plain, E16 two delayed, E17 `wor`; all three oracles restore `0001` after the release): once `md_nets` admits a delayed member, each delayed driver's restored output must feed the release's resolution
