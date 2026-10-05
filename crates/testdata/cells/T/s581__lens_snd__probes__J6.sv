module top;
  localparam logic [1:0][3:0] M = {4'hA, 4'h5};
  case (4'hA)
    M[1]: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("J6 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("J6 def %0d", w); end
  endcase
endmodule
