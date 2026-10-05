module top;
  logic [3:0] a;
  logic [3:0] v1, v2;
  function automatic logic [3:0] f(input logic [3:0] x);
    if (x === 4'bzzzz) $display("never");
    f = x + 1;
  endfunction
  wire [3:0] y = f(a);
  always @* begin v1 = y; $display("AS y=%b t=%0t", y, $time); end
  always_comb begin v2 = y; $display("AC y=%b t=%0t", y, $time); end
  initial a = 1;
  initial #1 $display("#1 v1=%b v2=%b", v1, v2);
  initial #3 $finish;
endmodule
