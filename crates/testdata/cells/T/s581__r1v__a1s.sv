`timescale 1ns/1ns
module t;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    case (i)
      0: begin : z wire [7:0] w = 8'd1; initial #1 $display("A1S zero %0d", w); end
      1: begin : o wire [7:0] w = 8'd2; initial #1 $display("A1S one %0d", w); end
      default: begin : d wire [7:0] w = 8'd99; initial #1 $display("A1S def %0d", w); end
    endcase
    localparam int K = i;
    initial #2 $display("A1D i=%0d K=%0d", i, K);
  end
endmodule
