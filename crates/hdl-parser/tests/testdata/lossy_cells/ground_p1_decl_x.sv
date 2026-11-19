module t;
  localparam logic [3:0] N = 4'b1x01;
  initial begin #1 $display("A n=%b", N); end
  initial #100 $finish;
endmodule
