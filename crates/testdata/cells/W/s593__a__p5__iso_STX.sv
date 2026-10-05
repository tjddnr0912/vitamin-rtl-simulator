`define EX (4'bx100 ==? 4'b1?00)
module t;
  struct packed { logic [`EX*3:0] f; } sx; initial #1 $display("STX %0d", $bits(sx));
  initial #3 $finish;
endmodule
