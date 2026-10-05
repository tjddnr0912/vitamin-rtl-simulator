package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pb::P;
  logic [P[3:0]:0] v;
  initial #1 $display("wid b=%0d", $bits(v));
  initial #100 $finish;
endmodule
