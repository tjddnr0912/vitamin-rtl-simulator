module t #(parameter P = 0);
  logic [1:0] r; int y;
  generate if (P == 1) begin : g
    always @(r) begin
      unique casez (r)
        2'b?1: y = 1;
        2'b1?: y = 2;
      endcase
    end
  end endgenerate
  initial begin
    y = 5;
    r = 2'b11;
    #1 $display("y=%0d", y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
