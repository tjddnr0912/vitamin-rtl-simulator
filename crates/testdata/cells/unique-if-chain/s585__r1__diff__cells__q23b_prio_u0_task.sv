module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0;
  task automatic t(input logic x, input logic z);
    priority if (x) r = 1;
    else unique0 if (z) r = 2;
  endtask
  initial begin
    #1 t(a, b);
    #1 b = 1; t(a, b);
    #1 $display("t=%0t r=%0d", $time, r);
    #1 $finish;
  end
endmodule
