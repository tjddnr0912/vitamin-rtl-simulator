module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [3:0] u [f(2)];
  localparam int B = $bits(u);
  initial begin #1 $display("B=%0d", B); $finish; end
endmodule
