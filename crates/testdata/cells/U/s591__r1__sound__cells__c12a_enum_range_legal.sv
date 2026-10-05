package pk; localparam S = 7; endpackage
module top;
  import pk::S;
  typedef enum {S[2]} t;
  localparam int K = S;
  initial #1 $display("er S=%0d K=%0d S1=%0d", S, K, S1);
  initial #100 $finish;
endmodule
