module top;
  generate
    case (1)
      65'd1: begin : g_a initial $display("C2 a"); end
      default: begin : g_def initial $display("C2 def"); end
    endcase
  endgenerate
endmodule
