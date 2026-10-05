package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  case (E1)
    0: begin : z initial #1 $display("gcs zero"); end
    1: begin : o initial #1 $display("gcs one"); end
    default: begin : d initial #1 $display("gcs def"); end
  endcase
  initial #100 $finish;
endmodule
