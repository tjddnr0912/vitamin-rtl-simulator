module top;
  logic a = 0, b = 0; logic [1:0] r = 0;
  initial begin
    #1 priority if (a) r = 1; else unique0 if (b) r = 2;
    #1 priority if (a) r = 1;
    #1 $finish;
  end
endmodule
