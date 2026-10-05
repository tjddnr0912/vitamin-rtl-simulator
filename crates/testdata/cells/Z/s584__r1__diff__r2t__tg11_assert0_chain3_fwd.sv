module top;
  logic [1:0] src = 2'd1;
  logic [1:0] p, m, y;
  always_comb p = src;
  always_comb m = p;
  always_comb begin
    y = m;
    assert #0 (m == 2'd1) else $error("A0 t=%0t m=%b", $time, m);
  end
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
