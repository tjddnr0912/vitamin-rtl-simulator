module top;
  case (-1)
    signed'(4'hF): begin : g_a wire [7:0] w = 8'd1; initial #1 $display("Z2 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("Z2 def %0d", w); end
  endcase
endmodule
