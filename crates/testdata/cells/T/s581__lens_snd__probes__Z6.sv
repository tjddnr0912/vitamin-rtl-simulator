module top;
  case (32'hFFFF_FFF0)
    (~4'd15): begin : g_a wire [7:0] w = 8'd1; initial #1 $display("Z6 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("Z6 def %0d", w); end
  endcase
endmodule
