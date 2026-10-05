program automatic p(input logic [1:0] r);
  int y;
  initial begin
    #1;
    if (r[0]) y = 1;
    else if (r[1]) y = 2;
    $display("y=%0d", y);
  end
endprogram
module top;
  logic [1:0] r = 2'b11;
  p u(.r(r));
  initial #100 $finish;
endmodule
