`define EX (4'bx100 ==? 4'b1?00)
module t;
  initial #1 $display("REPX %b", {(`EX + 1){1'b1}});
  initial #3 $finish;
endmodule
