class D;
  logic [1:0] r;
  function new(input logic x, input logic z);
    r = {z, x};
  endfunction
endclass
class C;
  D d;
  int n;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b dnull=%0d n=%0d", $time, x, z, d == null, n);
    n = n + 1;
    d = new(x, z);
    return {x, z};
  endfunction
endclass
module dut(input logic a, input logic b, output logic [1:0] y);
  C obj; initial begin $display("dut init"); obj = new; end
  assign y = obj.f(a, b);
  initial #1 $display("t=%0t n=%0d", $time, obj.n);
endmodule
module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    $display("top init");
    a = 0; b = 1;
    #2 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
