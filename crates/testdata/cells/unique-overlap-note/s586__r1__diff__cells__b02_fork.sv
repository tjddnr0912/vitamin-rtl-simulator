module top;
  logic [1:0] r = 2'b11; int y;
  initial #100 $finish;
  initial begin
    #1;
    fork
      unique casez (r)
        2'b?1: y = 1;
        2'b1?: y = 2;
      endcase
      #2 $display("fork side");
    join
    $display("y=%0d", y);
  end
endmodule
