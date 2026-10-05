module sub #(parameter logic [64:0] P = 0);
  generate
    case (1)
      P: begin : g_a initial $display("O08 a"); end
      default: begin : g_def initial $display("O08 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P({1'b1, 64'd1})) u();
endmodule
