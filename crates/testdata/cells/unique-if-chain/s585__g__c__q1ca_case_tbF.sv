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
module dut(input logic a, input logic b, output logic [1:0] y);
  always_comb begin
    y = 0;
    unique case ({a,b}) 2'b10: y = 1; 2'b01: y = 2; endcase
  end
endmodule
