module dut(input logic a, input logic b, output logic [1:0] y);
  assign y = 0;
  always begin : chk
    logic x, z; x = a; z = b;
    unique if (x) begin end else if (z) begin end
    @(a or b);
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
