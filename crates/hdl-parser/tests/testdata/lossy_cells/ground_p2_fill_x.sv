module t;
  logic [127:0] v;
  initial begin v = 'x; #1 $display("A v=%h", v); end
  initial #100 $finish;
endmodule
