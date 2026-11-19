module t;
  localparam logic [69:0] W = 70'bx1;
  initial begin #1 $display("A w=%b", W[3:0]); end
  initial #100 $finish;
endmodule
