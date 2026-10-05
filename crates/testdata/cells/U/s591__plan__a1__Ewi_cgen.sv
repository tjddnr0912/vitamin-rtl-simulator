package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  typedef enum {E0, E1} e_t;
  sgen #(.N(E1)) u3 ();
  initial #100 $finish;
endmodule
module sgen #(parameter N = 0) ();
  if (N == 1) begin : one initial #3 $display("gif %m one"); end else begin : oth initial #3 $display("gif %m other"); end
  case (N) 0: begin : c0 initial #3 $display("gcs %m zero"); end 1: begin : c1 initial #3 $display("gcs %m one"); end default: begin : cd initial #3 $display("gcs %m def"); end endcase
  for (genvar k = 0; k <= N; k++) begin : lp initial #3 $display("gfor %m k=%0d", k); end
endmodule
