module t;
  logic [3:0] r; int y;
  initial begin
    r = 4'd5;
    #1;
    unique case (r) inside
      [0:7]: y = 1;
      [4:9]: y = 2;
    endcase
    $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
