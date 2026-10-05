module child;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [f(2):0] v = '1;
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [f(2):0] v;
  logic [f(2):-2] w;
  logic [7:0] arr [f(3)];
  child u();
  initial begin
    #1 $display("bv=%0d l=%0d r=%0d s=%0d i=%0d d=%0d sa=%0d bw=%0d rw=%0d hv=%0d hs=%h", $bits(v), $left(v), $right(v), $size(v), $increment(v), $dimensions(arr), $size(arr), $bits(w), $right(w), $bits(u.v), u.v[f(2):4]);
    $finish;
  end
endmodule
