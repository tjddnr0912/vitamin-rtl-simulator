module top;
  logic a = 0, b = 0, c = 0; integer x = 0;
  initial begin
    #1 unique if (a) x = 1; else assert #0 (b) else if (c) $display("t=%0t c-branch", $time);
    $display("t=%0t D1 done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
