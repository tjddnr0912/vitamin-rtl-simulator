package p;
  parameter logic [15:0] A = 16'hB79F;
  parameter logic [15:0] B = 16'h1234;
  parameter logic [15:0] Z = 16'h0;
  typedef logic [79:0] st_t;
  typedef enum st_t {
    S0 = 80'h0,
    S1 = 80'hb79f1234b79f1234b79f,
    S2 = 80'h123412341234b79fb79f
  } st_e;
endpackage
module t;
  p::st_e s;
  initial begin s = p::S1; #1 $display("A s=%h s2=%h", s, p::S2); $finish; end
endmodule
