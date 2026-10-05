module sub #(parameter M = 2) ();
  localparam N = M * 2;
  localparam L = ((({N{1'b0}} + 8'd255 + 8'd1) >> 1) == 128);
  initial $display("R: M=%0d L=%0d", M, L);
endmodule
module t;
  sub #(.M(8)) u1();
  initial #10 $finish;
endmodule
