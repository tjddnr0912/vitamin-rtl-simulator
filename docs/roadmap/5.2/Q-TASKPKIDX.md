(§5.2 Q-TASKPKIDX · rank ① · source row 29 r3 build (W/row29/r3/dev/p1), RULINGS-r3build) (a packed index reading a module variable in an automatic task writes element 0):
* symptom: a packed element index that reads a module variable inside a framed (automatic) task lowers to element 0 when the target is a slot of the task's window: `int i = 1; task automatic t(); logic [1:0][15:2] pb; pb = '0; pb[i] = 14'h3fff; $display("R %h", pb); endtask` (b_whole) prints `R 0003fff` on PRE 719e37d2, sv2v `R fffc000`
* pins: a task-local `int i` (h_auto_local_i) and a module target (l_mod_pb) print `R fffc000`, and the read `pb[i]` in the same task is right (e_auto_rd `R 00d0 34`)
* note: row 29 r3 kept such targets on PRE's accept set (`net_in_task_frame`), r1 / r2 admitted them and wrote element 0 (`pb[i][9:4]` `R 00000d0`, sv2v `R 0340000`)
* status: silent-wrong, pre-existing
