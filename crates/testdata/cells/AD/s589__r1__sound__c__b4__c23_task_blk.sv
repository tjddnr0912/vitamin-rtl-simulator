package p;
  localparam int W = 3;
  task t(output int o); begin : b logic [W:0] x; x = '1; o = x; end endtask
endpackage
module top;
  import p::t;
  localparam int W = 7;
  int v;
  initial begin t(v); $display("v=%0d", v); end
  initial #100 $finish;
endmodule
