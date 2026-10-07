package a; parameter int D = 7; endpackage
package b; import a::D; export a::D; parameter int E = D + 1; endpackage
module t;
  import b::*;
  initial begin #1 $display("A E=%0d D=%0d", E, D); $finish; end
endmodule
