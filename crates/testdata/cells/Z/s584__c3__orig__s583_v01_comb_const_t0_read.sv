module top;
  logic [1:0] y;
  logic a = 1;
  logic z;
  always_comb y = 2'd3;
  always_comb z = a;
  initial begin
    $display("init t=%0t y=%b z=%b", $time, y, z);
    #0 $display("init#0 t=%0t y=%b z=%b", $time, y, z);
    #0 $display("init#0#0 t=%0t y=%b z=%b", $time, y, z);
  end
  initial #1 $finish;
endmodule
