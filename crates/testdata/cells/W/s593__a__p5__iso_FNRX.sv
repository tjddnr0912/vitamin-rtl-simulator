`define EX (4'bx100 ==? 4'b1?00)
module t;
  function automatic logic [`EX*3:0] fr(); return '1; endfunction initial #1 $display("FNRX %0d", $bits(fr()));
  initial #3 $finish;
endmodule
