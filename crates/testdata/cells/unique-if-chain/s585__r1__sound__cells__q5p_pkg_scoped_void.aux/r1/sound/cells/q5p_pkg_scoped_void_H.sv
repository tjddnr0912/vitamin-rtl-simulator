package pk;
  function void fv(input logic x, input logic z, output logic [1:0] r);
    r = 0;
    if (x) r = 1; else unique if (z) r = 2;
  endfunction
  task automatic t(input logic x, input logic z, output logic [1:0] r);
    r = 0;
    if (x) r = 1; else unique if (z) r = 2;
  endtask
endpackage
module top;
  logic a = 0, b = 0; logic [1:0] q;
  initial begin
    #1 pk::fv(a, b, q); $display("t=%0t fv q=%0d", $time, q);
    #1 pk::t(a, b, q); $display("t=%0t t q=%0d", $time, q);
    #1 b = 1; pk::fv(a, b, q); $display("t=%0t fv q=%0d", $time, q);
    #1 $finish;
  end
endmodule
