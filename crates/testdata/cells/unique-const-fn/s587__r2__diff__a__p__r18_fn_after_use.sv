module top;
  localparam int P = f(2);
  logic [f(2):0] v;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  initial begin #1 $display("P=%0d bv=%0d", P, $bits(v)); $finish; end
endmodule
