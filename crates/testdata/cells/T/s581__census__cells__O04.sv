module sub #(parameter logic [7:0] P = 0);
  generate
    case (P)
      -1: begin : g_a initial $display("O04 a"); end
      default: begin : g_def initial $display("O04 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(-1)) u();
endmodule
