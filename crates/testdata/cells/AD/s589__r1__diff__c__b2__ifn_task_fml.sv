package r;
  localparam int K = 8;
  function automatic int h(); return K; endfunction
endpackage
package p;
  import r::h;
  localparam int K = 3;
  task automatic t(output logic [h()-1:0] o); o = '1; endtask
endpackage
module top;
  import r::*;
  import p::t;
  logic [31:0] v;
  initial begin #1 v = 0; t(v); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
