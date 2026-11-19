module t; localparam logic [3:0] N = 'x; localparam logic [69:0] M = 'z;
  initial begin #1 $display("A n=%b m=%h", N, M); end
  initial #100 $finish;
endmodule
