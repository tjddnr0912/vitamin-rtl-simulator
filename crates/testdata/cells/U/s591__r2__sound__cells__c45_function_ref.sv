package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  function automatic int f(); return P; endfunction
  import pb::*;
  initial #1 $display("fn f=%0d P=%0d", f(), P);
  initial #100 $finish;
endmodule
