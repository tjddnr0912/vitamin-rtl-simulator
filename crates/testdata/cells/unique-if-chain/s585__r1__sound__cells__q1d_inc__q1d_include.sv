module top;
  logic a = 0, b = 0;
`include "chain.svh"
  initial begin
    #1 tc(a, b);
    $display("t=%0t inc-task", $time);
    #1 unique if (a) $display("a"); else if (b) $display("b");
    $display("t=%0t main", $time);
    #1 $finish;
  end
endmodule
