package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::P;
  import pa::P;
  initial #1 $display("d02 P=%0d b=%0d", P, $bits(P));
  initial #100 $finish;
endmodule
