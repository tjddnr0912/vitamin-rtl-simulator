module t #(parameter logic [3:0] B = 0) ();
  initial begin #1 $display("A b=%b", B); end
  initial #100 $finish;
endmodule
