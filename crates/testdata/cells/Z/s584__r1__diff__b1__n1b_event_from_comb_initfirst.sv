module top;
  logic [1:0] a = 2'd1;
  logic [1:0] y;
  event e;
  initial begin @e; $display("E1 t=%0t y=%b", $time, y); end
  always_comb begin y = a; -> e; end
  initial #10 $finish;
endmodule
