module sub #(parameter P = 0);
  generate
    case (P)
      8'hFF: begin : g_a initial $display("O13 a"); end
      default: begin : g_def initial $display("O13 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(8'hFF)) u();
endmodule
