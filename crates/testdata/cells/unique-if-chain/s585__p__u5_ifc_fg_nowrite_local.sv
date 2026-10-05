interface I;
  logic a, b; logic [1:0] y;
  function void g(input logic x, input logic z);
    unique if (x) begin end else if (z) begin end
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    g(x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
endinterface
module top;
  I i();
  initial begin
    i.a = 0; i.b = 1;
    #1 $display("t=%0t y=%0d", $time, i.y);
    #1 i.b = 0;
    #1 $display("t=%0t y=%0d", $time, i.y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
