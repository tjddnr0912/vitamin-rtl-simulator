module top;
  typedef enum {E0, E1} e_t;
  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction
  localparam integer W = fae(E1);
  initial #3 $display("pae %m W=%0d", W);
  initial #100 $finish;
endmodule
