module top;
  localparam logic [7:0] S = 8'hA5;
  case (2'b01)
    S[3 -: 2]: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("J5 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("J5 def %0d", w); end
  endcase
endmodule
