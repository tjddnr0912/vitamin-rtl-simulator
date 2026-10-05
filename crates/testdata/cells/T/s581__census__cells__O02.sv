module sub #(parameter logic signed [3:0] P = 0);
  generate
    case (P)
      4'b1111: begin : g_a initial $display("O02 a"); end
      default: begin : g_def initial $display("O02 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(-1)) u();
endmodule
