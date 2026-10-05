package pk; localparam [64:0] K = 65'd7; endpackage
module top;
  localparam [64:0] K = 65'd5;
  case (7)
    pk::K: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("NA13L a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("NA13L def %0d", w); end
  endcase
  case (5)
    K: begin : g_b wire [7:0] w = 8'd2; initial #1 $display("NA13L b %0d", w); end
    default: begin : g_bdef wire [7:0] w = 8'd3; initial #1 $display("NA13L bdef %0d", w); end
  endcase
endmodule
