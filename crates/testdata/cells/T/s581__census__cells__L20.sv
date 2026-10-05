module top;
  generate
    case (1)
      65'd1: begin : g_a initial $display("L20 a"); end
      default: begin : g_def initial $display("L20 def"); end
    endcase
  endgenerate
endmodule
