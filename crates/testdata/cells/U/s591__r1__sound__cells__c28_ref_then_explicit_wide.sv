package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  localparam int A = P;
  import pb::P;
  if (P > 65'd100) begin : big initial #1 $display("xw big"); end
  else begin : sm initial #1 $display("xw small"); end
  initial #1 $display("xw A=%0d P=%0d", A, P);
  initial #100 $finish;
endmodule
