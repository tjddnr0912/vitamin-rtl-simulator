module top;
  logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x);
    if (x === 4'bzzzz) $display("never");
    f = x + 1;
  endfunction
  wire [3:0] y = f(a);
  wire [3:0] v = y + 1;
  initial $monitor("M t=%0t y=%b v=%b", $time, y, v);
  initial begin a = 1; $strobe("S t=%0t y=%b v=%b", $time, y, v); #2 a = 3; end
  initial #5 $finish;
endmodule
