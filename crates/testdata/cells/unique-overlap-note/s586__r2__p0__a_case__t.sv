module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b11; y = 9;
    #1;
    priority0 case (r)
      2'b00: y = 0;
      2'b01: y = 1;
    endcase
    $display("y=%0d", y);
    #1 $finish;
  end
endmodule
