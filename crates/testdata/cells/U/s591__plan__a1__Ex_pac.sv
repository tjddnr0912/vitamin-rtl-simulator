module top;
  typedef enum {E0, E1} e_t;
  function automatic integer fc(input integer x); case (x) 0: fc = 3; 1: fc = 5; default: fc = 7; endcase endfunction
  wire [7:0] r = {fc(E1){1'b1}};
  initial #3 $display("pac %m r=%b", r);
  initial #100 $finish;
endmodule
