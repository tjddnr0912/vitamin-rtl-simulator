function automatic int f(input int a); return 3; endfunction
function automatic logic [f(2):0] h(input int x); return x; endfunction
module top;
  function automatic int f(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
  localparam int P = h(1000);
  int v;
  initial begin v = h(1000); $display("P=%0d v=%0d", P, v); #1 $finish; end
endmodule
