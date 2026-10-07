module top;
  int w;
  bit ok;
  initial begin
    ok = std::randomize(w) with { w dist { 0 := 10, [1:3] :/ 2 }; };   // scope randomize with an inline constraint
    $display("ok=%0d inrange=%0d", ok, (w >= 0 && w <= 3));
    $finish;
  end
endmodule
