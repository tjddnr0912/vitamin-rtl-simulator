package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
import pa::P;
module top;
  import pb::P;
  if (P > 65'd100) begin : big initial #1 $display("cuenwg big"); end else begin : sm initial #1 $display("cuenwg small"); end
  initial #100 $finish;
endmodule
