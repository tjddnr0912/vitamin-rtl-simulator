module top;
  localparam logic [3:0] A [0:1] = '{4'd1, 4'd2};
  case (2)
    A[1]: begin : g_a wire [7:0] w = 8'd1; initial #1 $display("J1 a %0d", w); end
    default: begin : g_def wire [7:0] w = 8'd99; initial #1 $display("J1 def %0d", w); end
  endcase
endmodule
