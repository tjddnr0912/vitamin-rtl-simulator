package p;
  parameter logic [15:0] A = 16'hB79F;
  parameter logic [15:0] B = 16'h1234;
  parameter logic [15:0] Z = 16'h0;
  typedef logic [79:0] st_t;
  typedef enum st_t {
    S0 = {Z, Z, Z, Z, Z},
    S1 = {A, B, A, B, A},
    S2 = {B, B, B, A, A}
  } st_e;
endpackage
module t;
  p::st_e s;
  initial begin s = p::S1; #1 $display("A s=%h s2=%h", s, p::S2); $finish; end
endmodule
