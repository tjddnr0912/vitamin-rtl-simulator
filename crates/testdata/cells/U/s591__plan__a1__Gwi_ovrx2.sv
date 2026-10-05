package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    subh #(.P(i[3:0])) ub ();
    subh #(.P({i, 1'b0})) uc ();
    subh #(.P(8'(i))) ud ();
  end
  initial #100 $finish;
endmodule
module subh #(parameter P = 0) ();
  initial #3 $display("ovrh %m P=%h b=%0d", P, $bits(P));
endmodule
