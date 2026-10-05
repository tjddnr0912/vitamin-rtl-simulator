module top;
  logic a, y, b, z;
  always @(posedge a or posedge y) $display("E t=%0t a=%b y=%b", $time, a, y);
  always_comb y = 1'b1;
  initial a = 1;
  // second pair: edge already seen at t0, then a comb woken later re-produces an edge
  always @(posedge b or posedge z) $display("F t=%0t b=%b z=%b", $time, b, z);
  always_comb z = b;
  initial b = 1;
  initial #5 $finish;
endmodule
