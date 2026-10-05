module top;
  case (8'd200)
    byte'(200): begin : g_a wire [7:0] w = 8'd1; initial #1 $display("Z5 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("Z5 def %0d", w); end
  endcase
endmodule
