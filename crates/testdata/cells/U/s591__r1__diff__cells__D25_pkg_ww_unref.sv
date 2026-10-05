package pa; localparam P = 3; localparam A = 1; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; localparam B = 2; endpackage
package pc;
  import pa::*;
  import pb::*;
  localparam Z = A + B;
endpackage
module top;
  initial #1 $display("d25 Z=%0d", pc::Z);
  initial #100 $finish;
endmodule
