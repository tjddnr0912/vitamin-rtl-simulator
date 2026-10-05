`define Q(k) uni``k
module top;
  logic [1:0] r = 2'b11; int y;
  initial #100 $finish;
  initial begin
    #1;
    y = 0; `Q(que) casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    $display("y=%0d", y);
  end
endmodule
