class C; let LX = 'x; logic [69:0] f; function void m(); f = LX; endfunction endclass
module t; C o; initial begin o = new; o.m(); #1 $display("A f=%h", o.f); end initial #100 $finish; endmodule
