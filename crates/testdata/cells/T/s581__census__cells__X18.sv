module top;
  generate
    case (1)
      65'bx: begin : g_a initial $display("X18 a"); end
      1: begin : g_b initial $display("X18 b"); end
      default: begin : g_def initial $display("X18 def"); end
    endcase
  endgenerate
endmodule
