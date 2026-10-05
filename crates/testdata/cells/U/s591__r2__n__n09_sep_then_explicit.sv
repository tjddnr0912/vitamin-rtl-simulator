package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pw; localparam [64:0] P = 65'h1_0000_0000_0000_000b; endpackage
module top;
  import pa::*;
  wire w;
  import pb::*;
  import pw::P;
  if (P > 65'd100) begin : big initial #1 $display("n09 big"); end
  else begin : sm initial #1 $display("n09 small"); end
  initial #1 $display("n09 P=%0d", P);
  initial #100 $finish;
endmodule
