module top;
  generate
    case (1)
      4'bx: begin : g_a initial $display("C3 a"); end
      default: begin : g_def initial $display("C3 def"); end
    endcase
  endgenerate
endmodule
