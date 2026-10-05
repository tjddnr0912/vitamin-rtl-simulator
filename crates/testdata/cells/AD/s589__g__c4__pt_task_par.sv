package q;
  localparam int W = 3;
  task automatic tk(input logic [W:0] x, output int o);
    o = x;
  endtask
endpackage
module top;
  import q::tk;
  localparam int W = 7;
  int v;
  initial begin tk(1000, v); $display("v=%0d", v); #1 $finish; end
endmodule
