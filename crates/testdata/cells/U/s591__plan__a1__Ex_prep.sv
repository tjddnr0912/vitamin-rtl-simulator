module top;
  typedef enum {E0, E1} e_t;
  wire [7:0] r = {(E1+1){1'b1}};
  localparam [7:0] RR = {(E1+1){1'b1}};
  initial #3 $display("prep %m r=%b RR=%b b=%0d", r, RR, $bits(E1));
  initial #100 $finish;
endmodule
