logic ua, ub;
function void g();
  unique case (1'b1) ua: ; ub: ; endcase
endfunction
class C;
  function logic [1:0] f(input logic x, input logic z);
    g();
    return {x, z};
  endfunction
endclass
module dut(input logic a, input logic b, output logic [1:0] y);
  C obj; initial obj = new;
  assign y = obj.f(a, b);
endmodule
module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    ua = 0; ub = 1; a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 ub = 0; b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
