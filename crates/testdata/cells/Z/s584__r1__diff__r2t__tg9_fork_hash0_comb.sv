module top;
  logic [1:0] a;
  logic [1:0] c = 2'd1;
  logic [1:0] y;
  always_comb y = a + c;
  initial fork
    begin #0 a = 2'd1; end
    begin @(y) $display("F t=%0t y=%b", $time, y); end
  join_none
  initial #1 begin $display("e t=%0t y=%b", $time, y); $finish; end
endmodule
