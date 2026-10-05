module top;
  logic [3:0] a = 4'b1010, b = 4'b0101; logic [1:0] r = 2'b11; int y; logic z;
  initial #100 $finish;
  initial begin
    #1 unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    z = (a+b)[0];
    $display("y=%0d z=%b", y, z);
  end
endmodule
