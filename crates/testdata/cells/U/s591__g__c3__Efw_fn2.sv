module top;
  localparam [64:0] E1 = 65'h1_0000_0000_0000_0000;
  function automatic int f();
    typedef enum {E0, E1} e_t;
    return E1 + 0;
  endfunction
  int r;
  initial begin
    #1 r = f();
    $display("fn2 f=%0d r=%0d", f(), r);
    #1 $display("post %0d", E1);
  end
  initial #100 $finish;
endmodule
