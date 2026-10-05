module top;
  logic [1:0] r;
  always_comb $display("comb t=%0t r=%b", $time, r);
  initial begin $display("init0"); #0 $display("init1"); #0 $display("init2"); r = 2'b01; $display("init3"); end
  initial #2 $finish;
  initial #100 $finish;
endmodule
