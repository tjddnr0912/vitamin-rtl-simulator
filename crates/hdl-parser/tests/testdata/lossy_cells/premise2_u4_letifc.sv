interface ifc; let LI = 'z; logic [69:0] s; initial s = LI; endinterface
module t; ifc i(); logic \let ; logic [69:0] w; initial begin \let = 1'b1; w = 0; #1 $display("A s=%h let=%b", i.s, \let ); end initial #100 $finish; endmodule
