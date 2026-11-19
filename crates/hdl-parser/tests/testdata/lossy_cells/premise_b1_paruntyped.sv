module t;
  localparam U = 'z;
  localparam bit [7:0] B8 = 'x;
  initial begin #1 $display("A u=%b ub=%0d b8=%b", U, $bits(U), B8); end
  initial #100 $finish;
endmodule
