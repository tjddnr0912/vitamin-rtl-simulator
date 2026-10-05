package pk;
  logic pa, pb;
  function void g();
    if (pa) ; else unique if (pb) ;
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    g();
    return {x, z};
  endfunction
endpackage
module dut(input logic a, input logic b, output logic [1:0] y);
  
  assign y = pk::f(a, b);
endmodule
module top;
    import pk::*;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    pa = 0; pb = 1; a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 pb = 0; b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
