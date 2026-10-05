package q;
  function automatic int f(input int a);
    case (a)
      2: f = 3;
      default: f = 9;
    endcase
  endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  import q::*;
  localparam int P = h(1000);
  int v;
  initial begin v = h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
  initial #50 $finish;
endmodule
