module top;
  integer seed = 7;
  logic [31:0] a;
  function automatic logic [31:0] f(input logic [31:0] x);
    f = $random(seed) ^ x;
  endfunction
  wire [31:0] y = f(a);
  integer r;
  initial begin a = 0; r = $random(seed); $display("init r=%0d", r); end
  initial #1 $display("y=%0d seed=%0d", y, seed);
  initial #3 $finish;
endmodule
