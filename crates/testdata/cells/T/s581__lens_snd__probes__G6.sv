module top;
  logic [3:0] m [0:(4'b1100 ==? {2'b1?, 2'b00})];
  initial begin $display("G6 %0d", $size(m)); end
endmodule
