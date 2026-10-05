package pk; localparam S1 = 7; endpackage
module top;
  import pk::S1;
  typedef enum {S[2]} t;
  localparam int K = S1;
  initial #1 $display("ec S1=%0d K=%0d", S1, K);
  initial #100 $finish;
endmodule
