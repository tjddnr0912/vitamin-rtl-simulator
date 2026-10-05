package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    leaf la[i:0] ();
  end
  initial #100 $finish;
endmodule
module leaf ();
  initial #3 $display("leaf %m");
endmodule
