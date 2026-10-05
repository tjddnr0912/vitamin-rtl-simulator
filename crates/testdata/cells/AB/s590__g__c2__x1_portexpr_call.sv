module dut(input logic [1:0] a, output logic [1:0] y);
  assign y = a;
endmodule
module top;
  logic p, q; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  dut u(.a(f(p, q)), .y(y));
  initial begin
    p = 0; q = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
