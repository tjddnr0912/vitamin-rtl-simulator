module top;
  case (4'b1111)
    $unsigned(-1): begin : g_a wire [7:0] w = 8'd1; initial #1 $display("Z8 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("Z8 def %0d", w); end
  endcase
endmodule
