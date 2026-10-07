package p;
  parameter logic [15:0] A = 16'hB79F;
  parameter logic [15:0] B = 16'h1234;
  typedef enum logic [31:0] { S0 = {A, B}, S1 = {B, A} } st_e;
endpackage
module t;
  p::st_e s;
  initial begin s = p::S1; #1 $display("A s=%h s0=%h", s, p::S0); $finish; end
endmodule
