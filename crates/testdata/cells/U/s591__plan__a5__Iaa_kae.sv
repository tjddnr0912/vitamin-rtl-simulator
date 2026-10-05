package pa; localparam P = 3; endpackage
package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pa::*;
  import pb::*;
  localparam [64:0] KW = P;
  sae #(.N(KW)) u6 ();
  initial #100 $finish;
endmodule
module sae #(parameter N = 0) ();
  function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; endfunction
  localparam integer W = fae(N);
  initial #3 $display("ae %m W=%0d", W);
endmodule
