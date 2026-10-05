module top;
  logic [1:0] src = 2'd1;
  logic en = 1'b1;
  logic [1:0] p, l, y;
  always_comb p = src;
  always_latch if (en) l = p;
  always_comb begin
    unique case (l)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  initial #1 $display("t=%0t y=%0d l=%0d", $time, y, l);
  initial #5 $finish;
endmodule
