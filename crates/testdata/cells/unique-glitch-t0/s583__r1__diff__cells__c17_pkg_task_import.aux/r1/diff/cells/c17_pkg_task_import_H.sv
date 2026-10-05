package pk;
  int cnt;
  task automatic pta(input logic x, input logic z, output int r);
    r = 0;
    if (x) r = 1; else unique if (z) r = 2;
  endtask
  task pts(input logic x, input logic z);
    if (x) cnt = 1; else priority if (z) cnt = 2;
  endtask
endpackage
module top;
  import pk::*;
  int v; logic p, q;
  initial begin
    p = 0; q = 0;
    #1 pta(p, q, v); $display("t=%0t pta v=%0d", $time, v);
    #1 pts(p, q); $display("t=%0t pts cnt=%0d", $time, cnt);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
