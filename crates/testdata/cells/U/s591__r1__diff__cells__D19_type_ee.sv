package pa; typedef logic [3:0] t; endpackage
package pb; typedef logic [7:0] t; endpackage
module top;
  import pa::t;
  import pb::t;
  t v;
  initial #1 $display("d19 b=%0d", $bits(v));
  initial #100 $finish;
endmodule
