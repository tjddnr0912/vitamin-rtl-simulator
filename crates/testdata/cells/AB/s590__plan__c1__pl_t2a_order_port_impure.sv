module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] g(input logic x, input logic z);
    $display("g t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = g(a, b);
endmodule
module top;
  logic a = 0, b = 1; logic [1:0] y1, y2; wire [1:0] y3 = {a, b};
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y1 = f(a, b);
  dut u(.a(a), .b(b), .y(y2));
  initial begin
    $display("i0 y1=%b y2=%b y3=%b", y1, y2, y3);
    #1 $display("t=%0t y1=%b y2=%b y3=%b", $time, y1, y2, y3);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
