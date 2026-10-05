module top;
  logic a, y1, y2; wire w, v;
  assign #0 w = a;
  assign v = y1 ^ y2;
  always_comb begin y1 = w; $display("C1 t=%0t w=%b", $time, w); end
  always_comb begin y2 = ~w; $display("C2 t=%0t w=%b y1=%b", $time, w, y1); end
  initial a = 1;
  initial #0 $display("Z t=%0t v=%b y1=%b y2=%b", $time, v, y1, y2);
  initial #5 $finish;
endmodule
