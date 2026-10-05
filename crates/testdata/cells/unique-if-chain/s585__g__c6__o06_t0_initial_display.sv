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
  initial begin $display("init0"); #0 $display("init1"); #0 $display("init2"); r = 2'b01; $display("init3"); end
  initial #2 $finish;
  initial #100 $finish;
endmodule
