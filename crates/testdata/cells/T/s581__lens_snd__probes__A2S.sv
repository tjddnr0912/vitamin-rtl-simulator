package pk;
  localparam [64:0] W = 65'h1_0000_0000_0000_0000;
endpackage
module top;
  import pk::*;
  for (genvar W = 0; W < 2; W++) begin : g
    case (W)
      0: begin : g_zero wire [7:0] w = 8'd1; initial #1 $display("A2S zero %m %0d", w); end
      1: begin : g_one  wire [7:0] w = 8'd2; initial #1 $display("A2S one %m %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("A2S def %m %0d", w); end
    endcase
  end
endmodule
