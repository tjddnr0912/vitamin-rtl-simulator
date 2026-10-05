module sub (input logic [f(2):0] p);
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  initial #1 $display("port=%0d", $bits(p));
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  typedef logic [f(2):0] t_t;
  function automatic logic [f(2):0] rf(input int a);
    rf = '1;
  endfunction
  function automatic int ff(input logic [f(2):0] x);
    ff = $bits(x);
  endfunction
  logic [f(2):0] pk;
  logic [3:0] un [f(2)];
  logic ur [f(2):0];
  t_t tv;
  logic [7:0] w = 8'h5a;
  sub u (.p(w));
  initial begin : b
    int arr [f(2)];
    #1 $display("pk=%0d td=%0d ret=%0d form=%0d", $bits(pk), $bits(tv), $bits(rf(0)), ff(0));
    $display("un size=%0d left=%0d right=%0d bits=%0d", $size(un), $left(un), $right(un), $bits(un));
    $display("ur size=%0d left=%0d right=%0d", $size(ur), $left(ur), $right(ur));
    $display("blk=%0d", $size(arr));
    #1 $finish;
  end
endmodule
