class C;
  int r;
  function new(input logic x, input logic z);
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  task t(input logic x, input logic z);
    unique if (x) r = 1; else if (z) r = 2;
  endtask
  function void v(input logic x, input logic z);
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  task t2(input logic x, input logic z);
    unique if (x) r = 1; else if (z) r = 2;
  endtask
endclass
module top;
  logic a = 0, b = 0;
  int q;
  function int fn(input logic x, input logic z);
    unique if (x) fn = 1; else if (z) fn = 2; else fn = 3;
    unique if (x) fn = 1; else if (z) fn = 2;
  endfunction
  task tk(input logic x, input logic z);
    unique if (x) q = 1; else if (z) q = 2;
  endtask
  function void fv(input logic x, input logic z);
    unique if (x) q = 1; else if (z) q = 2;
  endfunction
  C obj;
  initial begin
    #1 obj = new(a, b);
    $display("t=%0t new", $time);
    #1 obj.t(a, b);
    $display("t=%0t t", $time);
    #1 obj.v(a, b);
    $display("t=%0t v", $time);
    #1 obj.t2(a, b);
    $display("t=%0t t2", $time);
    #1 q = fn(a, b);
    $display("t=%0t fn q=%0d", $time, q);
    #1 tk(a, b);
    $display("t=%0t tk", $time);
    #1 fv(a, b);
    $display("t=%0t fv", $time);
    #1 unique if (a) q = 1; else if (b) q = 2;
    $display("t=%0t proc", $time);
    #1 $finish;
  end
endmodule
