module top;
  localparam UP = 4'sb1111;
  generate
    case (UP)
      4'sb1111: begin : g_a initial $display("S45 a"); end
      default: begin : g_def initial $display("S45 def"); end
    endcase
  endgenerate
endmodule
