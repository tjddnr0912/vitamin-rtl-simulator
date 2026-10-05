package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
package pc;
  import pa::*;
  import pb::*;
  function automatic int f(); return P; endfunction
endpackage
module top;
  localparam P = 9;
  localparam int K = pc::f();
  initial #1 $display("pfc f=%0d K=%0d", pc::f(), K);
  initial #100 $finish;
endmodule
