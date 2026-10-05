class K;
  int r;
  function void m(input logic x, input logic z);
    if (x) r = 1; else unique if (z) r = 2;
  endfunction
  task tm(input logic x, input logic z);
    if (x) r = 1; else unique if (z) r = 2;
  endtask
endclass
module top;
  logic [1:0] y; K k; logic p, q;
  function automatic void fv(input logic x, input logic z);
    if (x) y = 1; else unique if (z) y = 2;
  endfunction
  initial begin
    k = new; p = 0; q = 0;
    #1 fv(p, q); $display("t=%0t fvoid", $time);
    #1 k.m(p, q); $display("t=%0t classfn", $time);
    #1 k.tm(p, q); $display("t=%0t classtask", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
