module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b01;
    #1;
    unique case (r)
      2'b00: y = 0;
      2'b01: y = 1;
      2'b10: y = 2;
      2'b11: y = 3;
    endcase
    $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
