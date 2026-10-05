module top;
  logic a, y; wire w;
  assign #0 w = a;
  always_comb begin y = w; $display("C t=%0t w=%b", $time, w); if (w === 1'b1) $finish; end
  initial a = 1;
endmodule
