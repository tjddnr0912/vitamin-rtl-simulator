module t;
  logic [1:0] r; int y;
  initial begin
    r = 2'b01;
    #1;
    unique case (r)
      2'b01: y = 1;
      2'b01: y = 2;
    endcase
    $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
