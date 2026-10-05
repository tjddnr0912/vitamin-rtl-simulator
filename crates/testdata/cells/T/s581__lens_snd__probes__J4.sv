module top;
  localparam logic [0:7] R = 8'hA5;
  case (4'hA)
    R[0:3]: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("J4 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("J4 def %0d", w); end
  endcase
endmodule
