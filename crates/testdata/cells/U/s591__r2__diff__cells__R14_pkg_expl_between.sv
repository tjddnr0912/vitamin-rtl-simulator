package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package px; localparam Q = 4; endpackage
package pd;
  import pa::*;
  import px::Q;
  import pb::*;
  localparam int Z = P;
endpackage
module top;
  initial #1 $display("r14 Z=%0d", pd::Z);
  initial #100 $finish;
endmodule
