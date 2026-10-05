module top;
  for (genvar i = 0; i < 2; i++) begin
    localparam [64:0] L = (i == 0) ? 65'h1_0000_0000_0000_0001 : 65'd1;
    case (1)
      L: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("B1U a %m %0d", w); end
      default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("B1U def %m %0d", w); end
    endcase
  end
endmodule
