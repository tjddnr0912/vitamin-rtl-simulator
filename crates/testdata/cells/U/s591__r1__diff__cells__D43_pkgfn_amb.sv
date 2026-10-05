package pa; localparam P = 3; endpackage
package pb; localparam P = 5; endpackage
package pc;
  import pa::*;
  import pb::*;
  function automatic int f(); return P; endfunction
endpackage
module top;
  initial #1 $display("d43 f=%0d", pc::f());
  initial #100 $finish;
endmodule
