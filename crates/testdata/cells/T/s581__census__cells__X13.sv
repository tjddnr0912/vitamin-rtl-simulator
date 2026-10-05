module top;
  generate
    case (1)
      4'b1100 inside {4'b1?00}: begin : g_a initial $display("X13 a"); end
      default: begin : g_def initial $display("X13 def"); end
    endcase
  endgenerate
endmodule
