module top;
  logic [1:0] r = 2'b11; logic a = 0, b = 1, c = 1; int y;
  initial #100 $finish;
  initial begin
    #1;
    priority casez (r)
      2'b?1: if (a) y = 1;
             else unique if (b) y = 2;
             else if (c) y = 3;
      2'b1?: y = 4;
    endcase
    $display("y=%0d", y);
  end
endmodule
