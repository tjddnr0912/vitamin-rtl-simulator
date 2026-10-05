package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pb::*;
  import pa::*;
  if (P > 65'd100) begin : t initial #1 $display("d40 gif big"); end
  else begin : e initial #1 $display("d40 gif small"); end
  initial #2 $display("d40 P=%0d", P);
  initial #100 $finish;
endmodule
