module top;
  logic [1:0] src = 2'd1;
  logic [1:0] p, m1, m2, y;
  always_comb begin y = m2; unique case (m2) 2'd1: ; 2'd2: ; endcase assert (m2 == 2'd1) else $error("D3 t=%0t m2=%b", $time, m2); end
  always_comb begin m2 = m1; assert (m1 == 2'd1) else $error("D2 t=%0t m1=%b", $time, m1); end
  always_comb begin m1 = p; assert (p == 2'd1) else $error("D1 t=%0t p=%b", $time, p); end
  always_comb begin p = src; assert (src == 2'd1) else $error("D0 t=%0t", $time); end
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
