module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
module dut(input logic a, input logic b, output logic [1:0] y);
  always_latch begin
    y = 0;
    unique if (a) y = 1; else if (b) y = 2;
  end
endmodule
