module top;
  typedef enum {E0, E1} e_t;
  srep #(.N(E1)) u4 ();
  initial #100 $finish;
endmodule
module srep #(parameter N = 0) ();
  wire [7:0] r = {(N+1){1'b1}};
  localparam [7:0] RR = {(N+1){1'b1}};
  initial #3 $display("rep %m r=%b RR=%b", r, RR);
endmodule
