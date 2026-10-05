`define IFM if
module top;
  logic a = 0, b = 0, c = 0, d = 0;
  logic [1:0] r = 0;
  initial begin
    #1 unique if (a) r = 1; else (* m *) if (b) r = 2;
    #1 unique if (a) r = 1; else `IFM (b) r = 2;
    #1 unique if (a) r = 1; else
`ifdef NOPE
       begin end
`else
       if (b) r = 2;
`endif
    #1 unique if (a) r = 1;
       else if (b) r = 2;
       else unique if (c) r = 3;
       else if (d) r = 0;
    #1 unique if (a) r = 1;
       else if (b) r = 2;
       else priority if (c) r = 3;
       else if (d) r = 0;
    #1 unique if (a) r = 1;
       else unique0 if (b) r = 2;
       else if (c) r = 3;
    #1 priority if (a) r = 1; else unique0 if (b) r = 2;
    #1 unique if (a) r = 1;
       else if (b) r = 2;
       else if (c) r = 3;
    #1 if (a) r = 1;
       else unique if (b) r = 2;
       else if (c) r = 3;
    #1 unique if (a) r = 1; else (* m *) unique if (b) r = 2; else if (c) r = 3;
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
endmodule
