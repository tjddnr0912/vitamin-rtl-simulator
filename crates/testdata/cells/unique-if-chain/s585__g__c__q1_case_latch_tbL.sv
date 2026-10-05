module dut(input logic a, input logic b, output logic [1:0] y);
  always_latch begin
    y = 0;
    unique case ({a,b}) 2'b10: y = 1; 2'b01: y = 2; endcase
  end
endmodule
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
