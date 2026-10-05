module top;
  int v = 3; int y;
  initial #100 $finish;
  initial begin
    #1;
    unique case (v) inside
      [0:3]: y = 1;
      [2:5]: y = 2;
    endcase
    $display("y=%0d", y);
  end
endmodule
