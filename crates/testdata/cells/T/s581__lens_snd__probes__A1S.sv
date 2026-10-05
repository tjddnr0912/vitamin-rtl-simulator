module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      0: begin : g_zero wire [7:0] w = 8'd1; initial #1 $display("A1S zero %m %0d", w); end
      1: begin : g_one  wire [7:0] w = 8'd2; initial #1 $display("A1S one %m %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("A1S def %m %0d", w); end
    endcase
  end
endmodule
