package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
package pb; localparam P = 3; endpackage
module top;
  import pb::P;
  import pa::*;
  case (P)
    65'h1_0000_0000_0000_0009: begin : hw initial #1 $display("gcs wide"); end
    3: begin : h3 initial #1 $display("gcs three"); end
    default: begin : d initial #1 $display("gcs def"); end
  endcase
  initial #100 $finish;
endmodule
