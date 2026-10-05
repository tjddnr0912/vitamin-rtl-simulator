package pk; localparam E1 = 7; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  if (E1 == 1) begin : t initial #1 $display("gif hit"); end
  else begin : e initial #1 $display("gif miss"); end
  initial #100 $finish;
endmodule
