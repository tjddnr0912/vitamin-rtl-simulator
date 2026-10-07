module top;
  int n = 7;
  logic v = 1;
  initial begin
    $display("%8s|%s", $sformatf("#%0d", n), v ? $sformatf("x=%h", 8'h3c) : "   ");   // $sformatf as a task argument and inside ?:
    $finish;
  end
endmodule
