module top;
  logic [1:0] a = 2'd1;
  logic [1:0] y;
  always_comb y = a;
  initial fork
    begin @(y) $display("F t=%0t y=%b", $time, y); end
    begin #0 $display("Z t=%0t y=%b", $time, y); end
  join_none
  initial #10 begin $display("end y=%b", y); $finish; end
endmodule
