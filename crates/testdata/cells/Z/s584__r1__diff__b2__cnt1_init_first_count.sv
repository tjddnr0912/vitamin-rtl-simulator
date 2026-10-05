module top;
  logic [1:0] a;
  logic [1:0] y;
  initial a = 2'd1;
  always_comb begin y = a; $display("C t=%0t a=%b", $time, a); end
  initial #1 $finish;
endmodule
