package p;
  localparam int K = 8;
  function automatic logic [g()-1:0] f(); return '1; endfunction
  task automatic t(output logic [g()-1:0] o); o = '1; endtask
  function automatic int g(); return K; endfunction
endpackage
module top;
  import p::t;
  localparam int K = 232;
  function automatic int g(); return 5; endfunction
  logic [31:0] v, w;
  initial begin #1 v = p::f(); w = 0; t(w); $display("v=%0d w=%0d", v, w); $finish; end
  initial #100 $finish;
endmodule
