module top;
  generate
    case (1)
      |4'b100x: begin : g_a initial $display("X10 a"); end
      default: begin : g_def initial $display("X10 def"); end
    endcase
  endgenerate
endmodule
