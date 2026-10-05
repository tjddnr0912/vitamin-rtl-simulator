module top;
  logic a, en; wire bus; wire q;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign bus = en ? f(a) : 1'bz;
  assign bus = 1'b0;
  assign q = bus;
  always @(q) $display("Q t=%0t q=%b", $time, q);
  initial begin
    en = 1; a = 1;
    #1 $display("t=%0t bus=%b q=%b", $time, bus, q);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
