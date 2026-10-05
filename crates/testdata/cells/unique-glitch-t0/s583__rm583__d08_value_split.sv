module top;
  logic [1:0] y;
  always_comb y = 2'd3;
  initial begin
    $display("init t=%0t y=%b", $time, y);
    #0 $display("init#0 t=%0t y=%b", $time, y);
    #0 $display("init#0#0 t=%0t y=%b", $time, y);
  end
  initial #1 $finish;
  initial #100 $finish;
endmodule
