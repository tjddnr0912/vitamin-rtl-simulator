module sub #(parameter N = 4) ();
  localparam L = ((({N{1'b0}} + 8'd255 + 8'd1) >> 1) == 8'd128);
  initial $display("R: N=%0d L=%0d", N, L);
endmodule
module t;
  sub u();
  defparam u.N = 16;
  initial #10 $finish;
endmodule
