package q;
  localparam int K = 3;
  function automatic int g(); return K; endfunction
endpackage
package p;
  import q::*;
  localparam int K = 9;
  task automatic t(output int o); logic [g():0] x; x = '1; o = x; endtask
endpackage
module top;
  import q::*;
  int v;
  initial begin p::t(v); $display("v=%0d", v); end
  initial #100 $finish;
endmodule
