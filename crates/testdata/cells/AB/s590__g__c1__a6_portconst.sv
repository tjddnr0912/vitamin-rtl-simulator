module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
endmodule
module top;
  logic [1:0] y;
  dut u(.a(1'b0), .b(1'b1), .y(y));
  initial begin
    $display("i0 y=%b", y);
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
