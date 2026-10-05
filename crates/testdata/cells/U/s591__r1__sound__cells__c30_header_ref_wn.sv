package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top import pb::*; #(parameter [64:0] X = P) ();
  import pa::*;
  if (P > 65'd100) begin : big initial #1 $display("hw big"); end
  else begin : sm initial #1 $display("hw small"); end
  initial #1 $display("hw X=%0d P=%0d", X, P);
  initial #100 $finish;
endmodule
