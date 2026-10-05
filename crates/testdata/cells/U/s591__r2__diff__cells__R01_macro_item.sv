`define DECL_A localparam int A = P;
package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  `DECL_A
  import pb::*;
  initial #1 $display("r01 A=%0d P=%0d", A, P);
  initial #100 $finish;
endmodule
