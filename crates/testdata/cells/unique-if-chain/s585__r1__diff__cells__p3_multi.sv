module top;
  import pk::*;
  logic a = 0, b = 1;
  logic [1:0] r;
  sub u(.a(a), .b(b));
  initial begin
    #1 fv(a, b, r);
    #1 b = 0;
    #1 fv(a, b, r);
    #1 tk(a, b);
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
