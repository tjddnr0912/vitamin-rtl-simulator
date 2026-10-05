module top;
  localparam [64:0] K = 65'h1_0000_0000_0000_0005;
  if (1) begin : gb
    case (1)
      (K == 65'h1_0000_0000_0000_0007): begin : g_a wire [7:0] w = 8'd1; initial #1 $display("A4F a %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("A4F def %0d", w); end
    endcase
    localparam [64:0] K = 65'h1_0000_0000_0000_0007;
  end
endmodule
