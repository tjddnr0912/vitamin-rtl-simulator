module top;
  logic [1:0] r = 2'b11; int y;
  initial #100 $finish;
  final begin
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    $display("final y=%0d", y);
  end
endmodule
