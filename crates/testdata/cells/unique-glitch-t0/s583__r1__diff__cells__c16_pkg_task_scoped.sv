package pk;
  int cnt;
  task automatic pta(input logic x, input logic z, output int r);
    r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endtask
  task pts(input logic x, input logic z);
    priority if (x) cnt = 1; else if (z) cnt = 2;
  endtask
endpackage
module top;
  int v; logic p, q;
  initial begin
    p = 0; q = 0;
    #1 pk::pta(p, q, v); $display("t=%0t pk::pta v=%0d", $time, v);
    #1 pk::pts(p, q); $display("t=%0t pk::pts cnt=%0d", $time, pk::cnt);
    #1 q = 1; pk::pta(p, q, v); $display("t=%0t pk::pta hit v=%0d", $time, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
