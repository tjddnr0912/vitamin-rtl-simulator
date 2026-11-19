module s(input bit [69:0] i); initial #2 $display("B i=%h", i); endmodule
module t; bit [69:0] b = 'x; int k = 'z; s u(.i('x));
  initial begin #1 $display("A b=%h k=%0d", b, k); end
  initial #100 $finish;
endmodule
