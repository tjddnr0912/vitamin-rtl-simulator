package pk; localparam E1 = 7; endpackage
module top;
  import pk::E1;
  function automatic int f();
    typedef enum {E0, E1} e_t;
    return E1;
  endfunction
  initial #1 $display("d09 f=%0d E1=%0d", f(), E1);
  initial #100 $finish;
endmodule
