module top;
  logic [1:0] s = 2'd1;
  logic [1:0] x, y;
  always_comb x = s;
  always_comb begin y = x; $display("Y t=%0t x=%b", $time, x); end
  initial #1 $finish;
endmodule
