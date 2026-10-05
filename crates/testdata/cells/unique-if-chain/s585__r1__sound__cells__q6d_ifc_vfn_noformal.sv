interface I(input logic a, input logic b);
  function void g();
    unique if (a) ; else if (b) ;
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    g();
    return {x, z};
  endfunction
endinterface
module dut(input logic a, input logic b, output logic [1:0] y);
  I i(.a(a), .b(b));
  assign y = i.f(a, b);
endmodule
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
