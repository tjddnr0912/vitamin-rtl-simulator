module top;
  logic a, b;
  logic [1:0] y1, y2, y3;
  dut u(.a(a), .b(b), .y1(y1), .y2(y2), .y3(y3));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d %0d %0d", $time, y1, y2, y3);
    #1 b = 0;
    #1 $display("t=%0t y=%0d %0d %0d", $time, y1, y2, y3);
    #1 $finish;
  end
endmodule
module dut(input logic a, input logic b, output logic [1:0] y1, output logic [1:0] y2, output logic [1:0] y3);
  task automatic inner(input logic x, input logic z, output logic [1:0] r);
    r = 0;
    unique if (x) r = 1;
    else if (z) r = 2;
  endtask
  task automatic outer(input logic x, input logic z, output logic [1:0] r);
    inner(x, z, r);
  endtask
  function automatic void fv(input logic x, input logic z, output logic [1:0] r);
    r = 0;
    for (int k = 0; k < 1; k++) begin
      unique if (x) r = 1;
      else if (z) r = 2;
    end
  endfunction
  always_comb outer(a, b, y1);
  always @* fv(a, b, y2);
  always_latch begin
    unique if (a) y3 = 1;
    else if (b) y3 = 2;
  end
endmodule
