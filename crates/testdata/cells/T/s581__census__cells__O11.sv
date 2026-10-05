module sub #(parameter P = 0);
  generate
    case (P)
      32'hFFFFFFFF: begin : g_a initial $display("O11 a"); end
      default: begin : g_def initial $display("O11 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(-1)) u();
endmodule
