(§5.2 Q-CLKDRV · rank ① · source ENG-1 r2 (K08r, K08i)) (a clocking output drives its signal before the first synchronous drive):
* symptom: a clocking output drives its holder at every clocking event before the first synchronous drive: a `real` never written drives `0.0` over the signal (verilator keeps 2.5), an `int` drives x
* IEEE 1800 §14.16
* fix: a per-clockvar written flag in the drive, both lanes
* note: size S–M (PRE = POST)
