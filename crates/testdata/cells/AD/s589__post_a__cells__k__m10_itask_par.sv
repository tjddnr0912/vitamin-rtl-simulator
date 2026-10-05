package q;
  localparam int W = 3;
  int pv = 0;
  task tk(input int x, output int o);
    logic [W:0] t;
    t = x + pv;
    o = t;
  endtask
endpackage
module top;
  import q::tk;
  localparam int W = 7;
  int v;
  initial begin tk(1000, v); $display("v=%0d", v); #1 $finish; end
  initial #50 $finish;
endmodule
