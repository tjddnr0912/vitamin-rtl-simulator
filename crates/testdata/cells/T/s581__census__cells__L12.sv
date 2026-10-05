module top;
  generate
    case (1)
      1.0: begin : g_a initial $display("L12 a"); end
      default: begin : g_def initial $display("L12 def"); end
    endcase
  endgenerate
endmodule
