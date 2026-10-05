module top;
  generate
    case (4'sb1000)
      4'd8: begin : g_a initial $display("S21 a"); end
      default: begin : g_def initial $display("S21 def"); end
    endcase
  endgenerate
endmodule
