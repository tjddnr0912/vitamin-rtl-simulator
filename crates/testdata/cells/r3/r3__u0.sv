module t;
  logic [1:0] r;
  logic [1:0] m;
  always_comb begin
    m = 2'd0;
    unique0 casez (r)
      2'b?1: m = 2'd1;
      2'b1?: m = 2'd2;
    endcase
  end
  initial begin
    #1 r = 2'b11;
    #1 $display("t=%0t m=%0d", $time, m);
    #1 r = 2'b00;
    #1 $display("t=%0t m=%0d", $time, m);
    #1 $finish;
  end
endmodule
