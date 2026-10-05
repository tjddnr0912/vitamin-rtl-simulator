`define ELSE_IF else if
`define CHAIN(c1,c2,v) unique if (c1) r = 1; else if (c2) r = v;
`define IFX if
`define ASRT(x) assert (x) else $display("assert-fail t=%0t", $time);
`define UQ unique
`define CHAINML(c1,c2) unique if (c1) r = 1; \
   else if (c2) r = 2;
module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  initial begin
    #1 unique if (a) r = 1; `ELSE_IF (b) r = 2;
    #1 `CHAIN(a, b, 2)
    #1 unique if (a) r = 1; else `IFX (b) r = 2;
    #1 unique if (a) r = 1; else `ASRT(b)
    #1 unique if (a) r = 1; else
`ifdef FOO
       begin end
`else
       if (b) r = 2;
`endif
    #1 unique if (a) r = 1; else
`ifndef FOO
       if (b) r = 2;
`endif
    #1 `UQ if (a) r = 1; else if (b) r = 2;
    #1 `CHAINML(a, b)
    #1 unique if (a) r = 1; else `CHAIN(b, c, 3)
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
