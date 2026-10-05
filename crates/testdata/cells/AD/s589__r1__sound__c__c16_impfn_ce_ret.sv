package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  function automatic logic [g():0] h(); h = '1; endfunction
endpackage
module top;
  import q::*;
  localparam int B = $bits(p::h());
  localparam longint P = p::h();
  initial begin $display("B=%0d P=%0d", B, P); end
  initial #100 $finish;
endmodule
