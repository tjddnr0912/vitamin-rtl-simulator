module sub #(parameter logic [64:0] P = 0);
  generate
    case (1)
      P: begin : g_a initial $display("O09 a"); end
      default: begin : g_def initial $display("O09 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(65'd1)) u();
endmodule
