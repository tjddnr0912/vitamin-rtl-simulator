package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pb; localparam P = 3; endpackage
module top;
  import pb::P;
  import pa::*;
  if (P > 65'd100) begin : t initial #1 $display("gif big"); end
  else begin : e initial #1 $display("gif small"); end
  initial #100 $finish;
endmodule
