package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  if (P == 3) begin : g3 initial #1 $display("r05 g3"); end
  import pb::*;
  initial #2 $display("r05 P=%0d", P);
  initial #100 $finish;
endmodule
