module tb;
  logic a, b;
  logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    $dumpfile("e4.vcd"); $dumpvars(0, tb);
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0; a = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 a = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
endmodule
module dut(input logic a, input logic b, output logic [1:0] y);
  logic [1:0] m;
  always_comb begin
    unique if (a) m = 2'd1;
    else if (b) m = 2'd2;
  end
  always_comb begin
    unique if (m == 2'd1) y = 2'd3;
    else if (m == 2'd2) y = 2'd0;
  end
endmodule
