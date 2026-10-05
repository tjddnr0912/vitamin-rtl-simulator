package pk; localparam logic [7:0] K = 8'd5; endpackage
module top;
  import pk::*;
  case (8'd5)
    K: begin : g initial #1 $display("@k"); end
    default: begin : g initial #1 $display("@def"); end
  endcase
  generate
    localparam logic [7:0] K = 8'd6;
  endgenerate
  initial #5 $finish;
endmodule
