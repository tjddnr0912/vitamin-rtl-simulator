module top;
  generate
    case (4'sb1111)
      4'd2: begin : g_z initial $display("W14 z"); end
      -1: begin : g_a initial $display("W14 a"); end
      default: begin : g_def initial $display("W14 def"); end
    endcase
  endgenerate
endmodule
