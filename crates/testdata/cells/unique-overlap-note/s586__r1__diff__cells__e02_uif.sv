module top;
  logic a = 1, b = 1; int y;
  initial #100 $finish;
  initial begin
    repeat (2) begin
      #1;
      unique if (a) y = 1;
      else if (b) y = 2;
      $display("y=%0d", y);
    end
  end
endmodule
