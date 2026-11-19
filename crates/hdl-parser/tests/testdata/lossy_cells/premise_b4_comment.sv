module t; logic [69:0] v;
  initial begin v = 4/*c*/'b1x01; #1 $display("A v=%h", v); end
  initial #100 $finish;
endmodule
