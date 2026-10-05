module dut(input logic a, input logic b, output logic [1:0] y);
  always_comb begin
    y = 0;
    unique case ({a,b}) 2'b10: y = 1; 2'b01: y = 2; endcase
  end
endmodule
module mid(input logic a, input logic b, output logic [1:0] y);
  logic a2, b2;
  always_comb begin a2 = a; b2 = b; end
  dut l(.a(a2), .b(b2), .y(y));
endmodule
module top;
  logic a, b; logic [1:0] y;
  mid u(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
