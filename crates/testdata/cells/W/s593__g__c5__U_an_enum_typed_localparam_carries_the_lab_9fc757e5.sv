module tb;
  typedef enum logic [1:0] {A=1, B=2, C=3} e_t;
  localparam e_t E = B;
  localparam int N = E;
  initial begin $display("DIGEST=%0d %s %0d %0d", E, E.name(), N, $bits(E)); #1 $finish; end
endmodule