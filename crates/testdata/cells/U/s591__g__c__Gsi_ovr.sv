package pk; localparam string i = "AB"; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    sub #(.P(i)) u ();
    sub2 #(.N(i)) u2 (.a('0));
  end
  initial #5 $display("post %s", i);
  initial #100 $finish;
endmodule

module sub #(parameter P = 0) ();
  initial #3 $display("ovr %m P=%0d b=%0d", P, $bits(P));
endmodule
module sub2 #(parameter N = 0) (input logic [N:0] a);
  initial #3 $display("port %m b=%0d", $bits(a));
endmodule
