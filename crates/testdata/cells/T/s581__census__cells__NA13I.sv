package pk; localparam [64:0] K = 65'd7; endpackage
module top;
  import pk::*;
  localparam [64:0] K = 65'd5;
  case (5)
    K: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("NA13I a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA13I def %0d", w); end
  endcase
endmodule
