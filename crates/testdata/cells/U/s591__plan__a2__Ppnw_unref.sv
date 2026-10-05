package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pc2;
  import pa::*;
  import pb::*;
  localparam int Z = 4;
endpackage
module top;
  initial #1 $display("ppnwu Z=%0d", pc2::Z);
  initial #100 $finish;
endmodule
