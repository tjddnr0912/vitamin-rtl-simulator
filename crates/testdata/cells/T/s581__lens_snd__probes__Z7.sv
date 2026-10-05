module top;
  case (-16)
    (~4'sd15): begin : g_a wire [7:0] w = 8'd1; initial #1 $display("Z7 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("Z7 def %0d", w); end
  endcase
endmodule
