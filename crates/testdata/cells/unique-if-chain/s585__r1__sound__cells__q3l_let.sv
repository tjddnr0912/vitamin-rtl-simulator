module top;
  logic a, b; logic [1:0] y;
  function void g(input logic x, input logic z); logic [1:0] r; unique if (x) r = 1; else if (z) r = 2; endfunction
  function logic [1:0] f(input logic x, input logic z); return {x, z}; endfunction
  let L(x, z) = f(x, z);
  assign y = L(a, b);
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
endmodule
