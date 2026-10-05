module top;
  case (-4)
    ((-4'sd8) >>> 1): begin : g_a wire [7:0] w = 8'd1; initial #1 $display("Z4 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("Z4 def %0d", w); end
  endcase
endmodule
