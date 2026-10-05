package p;
  function automatic int g(); return 8; endfunction
  function automatic logic [31:0] h(input logic [31:0] a = {g(){1'b1}}); return a; endfunction
endpackage
package q;
  function automatic int g(); return 4; endfunction
  task automatic h(output logic [31:0] o, input logic [31:0] a = {g(){1'b1}}); o = a; endtask
endpackage
module top;
  import q::h;
  function automatic int g(); return 2; endfunction
  logic [31:0] v, w;
  initial begin #1 v = p::h(); w = 0; h(w); $display("v=%0d w=%0d", v, w); $finish; end
  initial #100 $finish;
endmodule
