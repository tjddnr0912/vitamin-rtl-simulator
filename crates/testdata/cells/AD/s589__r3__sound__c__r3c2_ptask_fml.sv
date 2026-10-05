package p;
  localparam int W = 3;
  function automatic int g(); return 3; endfunction
  task automatic t(input logic [W:0] a, output int o); o = a; endtask
endpackage
module top;
  import p::t;
  localparam int W = 7;
  function automatic int g(); return 7; endfunction
  int v;
  initial begin t(-1, v); $display("v=%0d", v); end
  initial #100 $finish;
endmodule
