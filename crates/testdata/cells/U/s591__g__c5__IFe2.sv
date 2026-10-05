package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
interface ifc;
  import pk::*;
  localparam int E1 = 1;
  localparam int K = E1;
  initial #1 $display("ife2 %m E1=%0d K=%0d", E1, K);
endinterface
module top;
  ifc x ();
  initial #100 $finish;
endmodule
