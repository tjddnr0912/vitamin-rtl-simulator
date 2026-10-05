module sub #(parameter P = 0);
  generate
    case (1)
      P: begin : g_a initial $display("O10 a"); end
      default: begin : g_def initial $display("O10 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(65'd1)) u();
endmodule
