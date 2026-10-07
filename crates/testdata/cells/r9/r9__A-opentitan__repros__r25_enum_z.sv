package p;
  typedef enum logic [2:0] { OK = 3'h0, ERR = 3'h1, UNDR = 'z } sts_e;
endpackage
module t;
  p::sts_e s;
  initial begin s = p::ERR; #1 $display("A s=%b undr=%b", s, p::UNDR); $finish; end
endmodule
