package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::i;
  for (genvar i = 0; i < 2; i++) begin : g
    sub #(.P(i + 1)) ua ();
    sub #(.P(i - 2)) ue ();
  end
  initial #100 $finish;
endmodule
module sub #(parameter P = 0) ();
  initial #3 $display("ovr %m P=%0d b=%0d", P, $bits(P));
endmodule
