module top;
  localparam UP = 4'sb1111;
  generate
    case (4'b1111)
      UP: begin : g_a initial $display("S36 a"); end
      default: begin : g_def initial $display("S36 def"); end
    endcase
  endgenerate
endmodule
