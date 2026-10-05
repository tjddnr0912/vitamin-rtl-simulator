module top;
  logic [1:0] src = 2'd1;
  logic [1:0] p, m, y;
  always_comb begin p = src; $display("P t=%0t src=%b", $time, src); end
  always_comb begin m = p; $display("M t=%0t p=%b", $time, p); end
  always_comb begin y = m; $display("Y t=%0t m=%b", $time, m); end
  initial begin #5 src = 2'd2; #5 src = 2'd3; #1 $finish; end
endmodule
