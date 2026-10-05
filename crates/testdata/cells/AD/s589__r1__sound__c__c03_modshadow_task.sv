package p;
  localparam int W = 3;
  task automatic t(output int o); logic [W:0] x; x = '1; o = x; endtask
endpackage
module top;
  import p::*;
  localparam int W = 7;
  task automatic t(output int o); logic [W:0] x; x = '1; o = x; endtask
  int v;
  initial begin t(v); $display("v=%0d", v); end
  initial #100 $finish;
endmodule
