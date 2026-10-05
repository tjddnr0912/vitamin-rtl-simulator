class D;
  logic [1:0] r;
  function new(input logic x, input logic z);
    r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
endclass
module top;
  logic a, b; D d;
  initial d = new(a, b);
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
  initial #100 $finish;
endmodule
