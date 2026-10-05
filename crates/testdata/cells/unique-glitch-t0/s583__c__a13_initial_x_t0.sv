module top;
  logic [1:0] r;
  logic [1:0] y;
  initial begin
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
    $display("t=%0t after x case", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
