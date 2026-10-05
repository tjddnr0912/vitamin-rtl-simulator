`define ELSE_IF else if
`define ELSE else
`define IF if
`define BR(s) s
module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  initial begin
    #1 unique if (a) r = 1; `ELSE_IF (b) r = 2;
    $display("t=%0t e1", $time);
    #1 unique if (a) r = 1; `ELSE `IF (b) r = 2;
    $display("t=%0t e2", $time);
    #1 unique if (a) r = 1; else `IF (b) r = 2; `ELSE begin end
    $display("t=%0t e3", $time);
    #1 unique if (a) r = 1; else `BR(if (b) r = 2;)
    $display("t=%0t e4", $time);
    #1 unique if (a) r = 1; `BR(else) if (b) r = 2; else `BR(if (c) r = 3;)
    $display("t=%0t e5", $time);
    #1 $finish;
  end
endmodule
