module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0;
  event e;
  initial begin
    #1 unique if (a) r = 1; else #1 if (b) r = 2;
    $display("t=%0t else-delay-if done", $time);
    fork #1 -> e; join_none
    #1 unique if (a) r = 1; else @(e) if (b) r = 2;
    $display("t=%0t else-event-if done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
