`define EX (4'bx100 ==? 4'b1?00)
module t;
  logic [7:0] v8 = 8'hA5; initial #1 $display("PSEL %b", v8[`EX*3+1:0]);
  initial #3 $finish;
endmodule
