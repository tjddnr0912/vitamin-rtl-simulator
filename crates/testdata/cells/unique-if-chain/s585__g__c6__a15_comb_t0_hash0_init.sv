module top;
  logic [1:0] r;
  logic [1:0] y;
  always_comb begin
    $display("eval t=%0t r=%b", $time, r);
    unique casez (r)
      2'b?1: y = 1;
      2'b1?: y = 2;
    endcase
  end
  initial #0 r = 2'b01;
  initial #2 $finish;
  initial #100 $finish;
endmodule
