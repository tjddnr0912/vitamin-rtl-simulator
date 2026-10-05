module sub #(parameter M = 8) ();
  localparam N = M * 2;
  localparam L = ((({N{1'b0}} + 8'd255 + 8'd1) >> 1) == 8'd128);
  initial $display("R: M=%0d L=%0d", M, L);
endmodule
module t;
  sub #(.M(2)) u1();
  initial #10 $finish;
endmodule
