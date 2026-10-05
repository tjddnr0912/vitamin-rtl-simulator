package p;
  localparam int K = 4;
  task automatic h(output logic [K-1:0] o); o = '1; endtask
endpackage
package q;
  localparam int K = 8;
  function automatic logic [K-1:0] h(); return '1; endfunction
endpackage
module top;
  import p::h;
  localparam int K = 16;
  logic [31:0] v, w;
  initial begin #1 w = 0; h(w); v = q::h(); $display("v=%0d w=%0d", v, w); $finish; end
  initial #100 $finish;
endmodule
