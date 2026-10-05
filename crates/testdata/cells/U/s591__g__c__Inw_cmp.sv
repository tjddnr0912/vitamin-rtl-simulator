package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  import pb::P;
  initial #1 $display("cmp gt=%0d eq3=%0d", (P > 65'd100), (P == 3));
  initial #100 $finish;
endmodule
