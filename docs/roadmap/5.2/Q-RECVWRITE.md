(§5.2 Q-RECVWRITE · rank ① · source ENG-2 r3 B2 re-plan OP-5) (a method writing its receiver counts as a read, never a write, in a read set):
* symptom: a method or system task that writes its receiver (`q.push_back(e)`, `q.pop_front()`, `q.insert`, `q.delete`, `s.putc`, `s.itoa`, `a.sort()`, `a.reverse()`, `c.randomize()`, and `q = new[n]`, lowered as a system task) is a read of the receiver in an implicit read set, never a write, for a local and a `ref` receiver alike
* status: pre-existing (PRE = POST)
* fix: IEEE 1800-2017 §9.2.2.2.1 exception b names what is "written within the block": decide whether a receiver write is one (and `q = new[n]`, an assignment), then route it through the read set's write collector
