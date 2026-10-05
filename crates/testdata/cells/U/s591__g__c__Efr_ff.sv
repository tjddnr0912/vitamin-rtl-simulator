module top;
  localparam real E1 = 2.5;
  int r;
  function automatic int f(input int a);
    typedef enum {E0, E1} e_t;
    int t;
    t = a;
    for (int k = 0; k < 1; k++) t = t + E1;
    return t;
  endfunction
  initial begin
    #1 r = f(10);
    $display("ff r=%0d", r);
    #1 $display("post %f", E1);
  end
  initial #100 $finish;
endmodule
