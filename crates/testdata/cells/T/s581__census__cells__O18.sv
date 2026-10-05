module sub #(parameter logic [7:0] P = 0);
  generate
    case (P)
      8'hFF: begin : g_a initial $display("O18 a"); end
      default: begin : g_def initial $display("O18 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub u();
  defparam u.P = -1;
endmodule
