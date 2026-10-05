module top;
  localparam logic [1:0] CFG = 2'd1;
  logic [1:0] p, m, y;
  always_comb p = CFG;
  always_comb m = p;
  always_comb begin
    y = m;
    assert (m == CFG) else $error("A t=%0t m=%b", $time, m);
  end
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
