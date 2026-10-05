package p;
  localparam int i = 5;
  function automatic int f(int x); return x; endfunction
  function automatic int h();
    int acc; acc = 0;
    for (int i = 0; i < 3; i++) acc += f(i);
    return acc;
  endfunction
endpackage
module top;
  localparam int L = p::h();
  int v;
  initial begin v = p::h(); $display("L=%0d v=%0d", L, v); end
  initial #100 $finish;
endmodule
