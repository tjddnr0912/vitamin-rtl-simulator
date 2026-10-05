module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
`include "p2_inc_chain.svh"
  initial begin
    #1 tk(a, b);
    #1 unique if (a) r = 1; else
`include "p2_inc_elseif.svh"
    #1 unique if (a) r = 1; else
`include "p2_inc_assert.svh"
    #1 unique if (a) r = 1; else if (c) r = 3;
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
