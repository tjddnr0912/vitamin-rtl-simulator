module top;
  localparam logic [11:4] Q = 8'hA5;
  case (4'h5)
    Q[7:4]: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("J8 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("J8 def %0d", w); end
  endcase
endmodule
