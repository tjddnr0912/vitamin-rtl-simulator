module t;
  logic priority0;
  initial begin
    priority0 = 1'b1;
    #1 $display("p=%0d", priority0);
    #1 $finish;
  end
endmodule
