class C;
  int n;
  function logic [31:0] f(input logic x);
    n = n + 1;
    return n;
  endfunction
endclass
module top;
  logic a; logic [31:0] y;
  C obj; initial obj = new;
  assign y = obj.f(a);
  initial begin
    a = 0;
    #1 $display("t=%0t y=%0d n=%0d", $time, y, obj.n);
    a = 1;
    #1 $display("t=%0t y=%0d n=%0d", $time, y, obj.n);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
