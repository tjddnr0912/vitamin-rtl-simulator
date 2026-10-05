package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module m import pa::*; (input logic [P-1:0] a);
  import pb::*;
  initial #1 $display("n06 P=%0d b=%0d", P, $bits(a));
endmodule
module top;
  logic [2:0] x;
  m u (.a(x));
  initial #100 $finish;
endmodule
