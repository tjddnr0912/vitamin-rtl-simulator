module top;
  if (1) begin : b
    localparam [127:0] k = ~128'd0;
    for (genvar k = 0; k < 2; k++) begin : g
      case (k)
        0: begin : g_zero wire [7:0] w = 8'd1; initial #1 $display("A3S zero %m %0d", w); end
        1: begin : g_one  wire [7:0] w = 8'd2; initial #1 $display("A3S one %m %0d", w); end
        default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("A3S def %m %0d", w); end
      endcase
    end
  end
endmodule
