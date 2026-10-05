package q;
  function automatic int f(input int a);
    f = 3;
    unique if (a == 1) f = 10;
  endfunction
  function automatic logic [f(2):0] g(input int a);
    g = f(a) + 1;
  endfunction
  parameter logic [f(2):0] PR = '1;
  localparam int PB = $bits(PR);
endpackage
module top;
  import q::*;
  localparam int P1 = q::g(2);
  localparam int P2 = f(2);
  localparam int P3 = g(2);
  initial begin #1 $display("P1=%0d P2=%0d P3=%0d PB=%0d PR=%h", P1, P2, P3, q::PB, q::PR); $finish; end
endmodule
