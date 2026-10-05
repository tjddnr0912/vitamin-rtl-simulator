module top;
  logic [1:0] a;
  logic [1:0] y;
  always_comb begin y = a; assert #0 (y == 2'd1) else $error("A0 t=%0t y=%b", $time, y); end
  initial a = 2'd1;
  initial #1 $finish;
endmodule
