module top;
  logic a = 0, b = 0, c = 0; integer x = 0;
  initial begin
    #1 unique if (a) x = 1; else if (b) x = 2; else assert (c);
    $display("t=%0t done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
