module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] f(input logic x, input logic z);
    $display("f %m t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
endmodule
module top;
  logic a, b; logic [1:0] y1, y2;
  dut u1(.a(a), .b(b), .y(y1));
  dut u2(.a(b), .b(a), .y(y2));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y1=%b y2=%b", $time, y1, y2);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
