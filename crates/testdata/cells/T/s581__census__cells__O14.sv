module sub #(parameter P = -1);
  generate
    case (P)
      4'b1111: begin : g_a initial $display("O14 a"); end
      default: begin : g_def initial $display("O14 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(-1)) u();
endmodule
