package p;
  localparam int K = 8;
  int sink;
  function automatic int g(); return K; endfunction
  task t(input logic [g()-1:0] a, output logic [31:0] o); sink = a; o = a; endtask
endpackage
module top;
  import p::t;
  localparam int K = 232;
  function automatic int g(); return 5; endfunction
  logic [31:0] w;
  initial begin #1 t(32'hFFFF_FFFF, w); $display("w=%0d", w); $finish; end
  initial #100 $finish;
endmodule
