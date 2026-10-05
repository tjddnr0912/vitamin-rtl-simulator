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
  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b10: f = 1; 2'b01: f = 2; endcase
  endfunction
  assign y = f(a, b);
endmodule
