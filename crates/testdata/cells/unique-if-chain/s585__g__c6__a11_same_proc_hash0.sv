module top;
  logic [1:0] r = 2'b00;
  logic [1:0] y;
  initial begin
    #5;
    $display("eval t=%0t r=%b", $time, r);
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    #0 r = 2'b01;
    $display("after #0 t=%0t", $time);
    #1 $display("t=%0t done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
