module top;
  logic a, kb = 1'b1, y, z; wire w;
  assign #0 w = a;
  always_comb begin y = w ^ z; $display("C t=%0t w=%b z=%b", $time, w, z); end
  always_comb begin z = kb; $display("D t=%0t kb=%b", $time, kb); end
  initial a = 1;
  initial #5 $display("F y=%b", y);
  initial #6 $finish;
endmodule
