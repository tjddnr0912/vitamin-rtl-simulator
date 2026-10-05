module top;
  generate
    case (1)
      1.0, 1: begin : g_a initial $display("M11 a"); end
      default: begin : g_def initial $display("M11 def"); end
    endcase
  endgenerate
endmodule
