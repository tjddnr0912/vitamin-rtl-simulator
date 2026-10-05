module top;
  logic [1:0] y; logic a = 0, b = 0;
  function automatic logic [1:0] cf(input logic x, input logic z);
    cf = 0;
    unique if (x) cf = 1; else if (z) cf = 2;
  endfunction
  logic [1:0] v = cf(0, 0);
  wire  [1:0] w = cf(0, 0);
  initial begin y = cf(0, 0); #1 $display("y=%0d v=%0d w=%0d", y, v, w); $finish; end
endmodule
