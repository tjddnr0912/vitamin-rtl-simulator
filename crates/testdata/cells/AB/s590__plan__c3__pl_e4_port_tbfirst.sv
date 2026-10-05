module top;
  logic a, b; wire [1:0] y;
  initial begin
    $display("i0 y=%b", y);
    a = 0; b = 1;
    @(y) $display("B t=%0t y=%b", $time, y);
  end
  initial begin
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  always @(posedge y[0]) $display("P t=%0t y=%b", $time, y);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  dut u(.a(a), .b(b), .y(y));
  initial #100 $finish;
endmodule
module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  assign y = f(a, b);
  always_comb if (y == 2'b11) $display("C bad");
endmodule
