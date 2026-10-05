module top;
  generate
    case (1)
      {64'd0, 1'bx}: begin : g_a initial $display("X17 a"); end
      1: begin : g_b initial $display("X17 b"); end
      default: begin : g_def initial $display("X17 def"); end
    endcase
  endgenerate
endmodule
