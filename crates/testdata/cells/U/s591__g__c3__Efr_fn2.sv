module top;
  localparam real E1 = 2.5;
  function automatic int f();
    typedef enum {E0, E1} e_t;
    return E1 + 0;
  endfunction
  int r;
  initial begin
    #1 r = f();
    $display("fn2 f=%0d r=%0d", f(), r);
    #1 $display("post %f", E1);
  end
  initial #100 $finish;
endmodule
