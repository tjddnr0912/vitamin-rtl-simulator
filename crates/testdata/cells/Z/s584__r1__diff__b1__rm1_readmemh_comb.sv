module top;
  logic [7:0] mem [0:3];
  logic [1:0] addr = 2'd2;
  logic [7:0] y;
  always_comb y = mem[addr];
  initial $readmemh("/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s584/r1/diff/b1/rm1.hex", mem);
  always @(y) $display("Y t=%0t y=%h", $time, y);
  initial #1 $display("t=%0t y=%h", $time, y);
  initial #10 $finish;
endmodule
