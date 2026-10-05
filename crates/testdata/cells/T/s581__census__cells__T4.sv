module top;
  generate
    case (1)
      4'b1100 inside {{2'b1?, 2'b00}}: begin : g_a initial $display("T4 a"); end
      default: begin : g_def initial $display("T4 def"); end
    endcase
  endgenerate
endmodule
