package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    logic [i+1:0] v;
    initial #1 $display("wid %m b=%0d", $bits(v));
  end
  initial #5 $display("post %0d", i);
  initial #100 $finish;
endmodule
