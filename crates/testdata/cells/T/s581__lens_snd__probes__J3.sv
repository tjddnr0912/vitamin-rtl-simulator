module top;
  localparam logic [11:4] Q = 8'hA5;
  case (4'hA)
    Q[11:8]: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("J3 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("J3 def %0d", w); end
  endcase
endmodule
