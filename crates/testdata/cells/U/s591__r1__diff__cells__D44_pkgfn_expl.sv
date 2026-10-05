package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc;
  import pa::*;
  import pb::P;
  localparam [64:0] Z = P;
  function automatic logic [64:0] f(); return P; endfunction
endpackage
module top;
  initial #1 $display("d44 Z=%0d f=%0d", pc::Z, pc::f());
  initial #100 $finish;
endmodule
