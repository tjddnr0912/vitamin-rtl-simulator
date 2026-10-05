module top;
  logic c1, c2, en;
  function automatic logic g(input logic c, input logic e);
    if (e === 1'bz) $display("never");
    g = c & e;
  endfunction
  wire g1w, g2w;
  logic g1l, g2l;
  assign g1w = g(c1, en);
  assign g2w = g(c2, en);
  assign g1l = g(c1, en);
  assign g2l = g(c2, en);
  always @(posedge g1w) $display("P1W t=%0t", $time);
  always @(negedge g1w) $display("N1W t=%0t", $time);
  always @(posedge g2w) $display("P2W t=%0t", $time);
  always @(negedge g2w) $display("N2W t=%0t", $time);
  always @(posedge g1l) $display("P1L t=%0t", $time);
  always @(negedge g1l) $display("N1L t=%0t", $time);
  always @(posedge g2l) $display("P2L t=%0t", $time);
  always @(negedge g2l) $display("N2L t=%0t", $time);
  initial begin c1 = 1; c2 = 0; en = 1; end
  always #5 begin c1 = ~c1; c2 = ~c2; end
  initial #12 $finish;
endmodule
