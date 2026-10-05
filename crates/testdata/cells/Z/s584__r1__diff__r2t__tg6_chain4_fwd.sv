module top;
  logic [1:0] src = 2'd1;
  logic [1:0] p, m1, m2, y;
  always_comb p = src;
  always_comb m1 = p;
  always_comb m2 = m1;
  always_comb begin
    unique case (m2)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
