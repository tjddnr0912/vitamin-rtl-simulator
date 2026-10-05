package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module m;
  import pa::*;
  import pb::P;
  initial #1 $display("m %m P=%0d b=%0d", P, $bits(P));
endmodule
module n;
  import pa::*;
  initial #1 $display("n %m P=%0d b=%0d", P, $bits(P));
endmodule
module top;
  m u1 ();
  n u3 ();
  m u2 ();
  initial #100 $finish;
endmodule
