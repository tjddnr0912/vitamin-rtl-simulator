package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pb::*;
  import pa::*;
  case (P)
    65'h1_0000_0000_0000_0009: begin : hw initial #1 $display("gcs wide"); end
    3: begin : h3 initial #1 $display("gcs three"); end
    default: begin : d initial #1 $display("gcs def"); end
  endcase
  initial #100 $finish;
endmodule
