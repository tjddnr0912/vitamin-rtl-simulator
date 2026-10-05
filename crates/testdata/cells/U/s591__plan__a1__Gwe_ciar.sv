package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::i;
  for (genvar i = 0; i < 2; i++) begin : g
    siar #(.N(i)) u5 ();
  end
  initial #100 $finish;
endmodule
module siar #(parameter N = 0) ();
  leaf lf[N:0] ();
endmodule
module leaf ();
  initial #3 $display("leaf %m");
endmodule
