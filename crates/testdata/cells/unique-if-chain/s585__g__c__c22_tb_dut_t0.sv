module dut(input logic a, input logic b, input logic [1:0] s, output logic [1:0] y, output logic [1:0] z);
  always_comb begin
    y = 0;
    unique if (a) y = 1; else if (b) y = 2;
  end
  always_comb begin
    z = 0;
    unique case (s) 2'd1: z = 1; 2'd2: z = 2; endcase
  end
endmodule
module top;
  logic a, b; logic [1:0] s, y, z;
  dut u(.a(a), .b(b), .s(s), .y(y), .z(z));
  initial begin
    a = 0; b = 1; s = 2'd1;
    #1 $display("t=%0t y=%0d z=%0d", $time, y, z);
    #1 b = 0; s = 2'd0;
    #1 $display("t=%0t y=%0d z=%0d", $time, y, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
