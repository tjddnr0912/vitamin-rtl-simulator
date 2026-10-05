package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  int sink;
  task t1(output int o, input int a = h()); sink = a; o = a; endtask
  task t2(output logic [31:0] o, input logic [31:0] a = {h(){1'b1}}); sink = a; o = a; endtask
endpackage
module top;
  import r::*;
  import p::t1; import p::t2;
  int u; logic [31:0] w;
  initial begin #1 t1(u); t2(w); $display("u=%0d w=%0d", u, w); $finish; end
  initial #100 $finish;
endmodule
