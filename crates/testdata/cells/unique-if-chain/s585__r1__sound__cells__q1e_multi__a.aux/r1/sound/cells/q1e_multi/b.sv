module top;
  logic a = 0, b = 0;
  sub u(.a(a), .b(b));
  initial begin
    #2 unique if (a) $display("ta"); else assert (b) else if (a) $display("tb");
    $display("t=%0t top", $time);
    #1 $finish;
  end
endmodule
