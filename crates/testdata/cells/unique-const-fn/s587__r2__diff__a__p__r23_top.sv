module top;
  function automatic int g(input int a);
    g = 7;
    if (a == 1) g = 10;
  endfunction
  localparam int P = g(2);
  localparam int R = q::f(2);
  logic [g(2):0] w;
  initial begin #1 $display("P=%0d R=%0d bw=%0d", P, R, $bits(w)); $finish; end
endmodule
