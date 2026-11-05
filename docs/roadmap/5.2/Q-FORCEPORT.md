(§5.2 Q-FORCEPORT · rank ① · source Q-RELDELAY plan (E10); r1 lens-diff F4 (L05 Q05 N10)) (a `force` on a net bound to a child's output port is invisible inside the child):
* symptom: a `force` on a net bound to a child's output port is not visible through the child's port name: E10 t=6 vita `o=0001`, iverilog = sv2v = verilator `o=0010` (L05 t=6 the same)
* note: the release side of a port-driven net is Q-RELIMM's (L05 Q05 N10)
* status: pre-existing
