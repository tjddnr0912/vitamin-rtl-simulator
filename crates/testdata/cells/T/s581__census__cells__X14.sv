module top;
  generate
    case (1)
      4'b1100 inside {4'b1100, 4'b0000}: begin : g_a initial $display("X14 a"); end
      default: begin : g_def initial $display("X14 def"); end
    endcase
  endgenerate
endmodule
