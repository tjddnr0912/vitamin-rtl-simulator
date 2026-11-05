(§3.b iface-port-forward · DEFERRED · in Nets / timing) (an interface port passed on to a child binds a 1-bit net):
* symptom: an interface port passed on to a child (`mid(bus_if.src bm); leaf u(.b(bm))`) binds an implicit 1-bit net (W2003, then E3002)
* cause: `ports.rs:258–285` `bind_iface_port` looks up `iface_insts` only
* oracles: verilator and sv2v `d=42` (iverilog parse error)
* witness: row-9 B2, EL2
* status: DEFERRED (§7 row 54)
