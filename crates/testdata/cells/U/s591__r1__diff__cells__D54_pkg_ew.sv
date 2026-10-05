package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; localparam Q = 4; endpackage
package pc;
  import pa::P;
  import pb::*;
  localparam int Z = P;
  localparam int Y = Q;
endpackage
module top;
  initial #1 $display("d54 Z=%0d Y=%0d", pc::Z, pc::Y);
  initial #100 $finish;
endmodule
