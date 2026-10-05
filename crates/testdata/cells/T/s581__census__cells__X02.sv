module top;
  generate
    case (1)
      1'bx: begin : g_a initial $display("X02 a"); end
      1: begin : g_b initial $display("X02 b"); end
      default: begin : g_def initial $display("X02 def"); end
    endcase
  endgenerate
endmodule
