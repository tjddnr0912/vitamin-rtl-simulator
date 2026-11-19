module t #(parameter int W = 8); logic [W-1:0] v = 'x; logic [W-1:0] v2; wire [W-1:0] n = 'z;
  initial begin v2 = 'z; #1 $display("A w=%0d v=%h v2=%h n=%h", W, v, v2, n); end
  initial #100 $finish;
endmodule
