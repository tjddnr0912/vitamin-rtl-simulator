(§3.b modport-port-actual · DEFERRED · in Nets / timing) (an interface modport as a port actual is refused):
* symptom: `sub u(w.mp)` is E3002
* witness: corpus witness row-9 B1 (EL2, `.p(ifc.mp)`, IEEE §25.5: `interface port b must be connected to an interface instance name`; verilator and sv2v `d=42`)
* cause: `ports.rs:258–285` `bind_iface_port` looks up `iface_insts` only
* oracles: verilator
* status: DEFERRED (§7 row 54)
