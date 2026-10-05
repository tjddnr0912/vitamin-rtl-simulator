module dut(input logic a, input logic b, output logic [1:0] y);
  always_comb begin
    y = 0;
    unique if (b) y = 2;
  end
endmodule
module top;
  logic ra, rb; wire a, b; logic [1:0] y;
  assign a = ra; assign b = rb;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    ra = 0; rb = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 rb = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
