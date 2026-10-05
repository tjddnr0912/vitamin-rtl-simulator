module top;
  logic [1:0] r = 2'b00;
  logic [1:0] y;
  initial begin
    #1 unique0 case (r)
      2'b01: y = 1;
      2'b10: y = 2;
    endcase
    $display("t=%0t case done", $time);
    #1 unique0 casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    $display("t=%0t casez done", $time);
    #1 unique0 casex (r)
      2'bx1: y = 1;
      2'b1x: y = 2;
    endcase
    $display("t=%0t casex done", $time);
    #1 r = 2'b01; unique0 case (r)
      2'b01: y = 1;
      2'b10: y = 2;
    endcase
    $display("t=%0t case-match done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
