package p;
  localparam int K = 8;
  function automatic int g(); return K; endfunction
  function automatic logic [31:0] f1(input logic [31:0] a = {g(){1'b1}}); return a; endfunction
  function automatic int f2(output logic [31:0] o, input logic [31:0] a = {g(){1'b1}}); o = a; return 0; endfunction
endpackage
module top;
  localparam int K = 232;
  function automatic int g(); return 5; endfunction
  logic [31:0] v, w; int r;
  initial begin #1 v = p::f1(); r = p::f2(w); $display("v=%0d w=%0d", v, w); $finish; end
  initial #100 $finish;
endmodule
