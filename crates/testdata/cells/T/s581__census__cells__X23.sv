module top;
  generate
    case (1)
      4'b1100 inside {4'b11x0}: begin : g_a initial $display("X23 a"); end
      default: begin : g_def initial $display("X23 def"); end
    endcase
  endgenerate
endmodule
