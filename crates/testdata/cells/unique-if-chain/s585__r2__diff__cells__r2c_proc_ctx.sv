interface ifc(input logic a, input logic b);
  initial #1 begin unique if (a) $display("i a"); else if (b) $display("i b"); end
  always @(a or b) begin priority if (a) $display("ia a"); else if (b) $display("ia b"); end
endinterface
program prg(input logic a, input logic b);
  initial #2 begin unique if (a) $display("p a"); else if (b) $display("p b"); end
endprogram
module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0;
  ifc u_if(.a(a), .b(b));
  prg u_p(.a(a), .b(b));
  task automatic tk(input logic x, input logic z);
    unique if (x) r = 1;
    else if (z) r = 2;
  endtask
  initial begin
    #3 fork
      begin unique if (a) r = 1; else if (b) r = 2; end
      begin #1 priority if (a) r = 1; else if (b) r = 2; end
    join_any
    #2 fork
      begin unique if (a) r = 1; else if (b) r = 2; end
    join_none
    #1 unique if (a) tk(a, b); else if (b) tk(a, b); else if (r == 3) tk(a, b);
    #1 tk(a, b);
    #1 a = 1; unique if (a) tk(0, 0); else if (b) r = 2;
    #1 b = 1; a = 0;
    #1 $finish;
  end
  final begin
    unique if (r == 3) $display("f3"); else if (r == 2) $display("f2");
  end
endmodule
