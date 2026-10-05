module top;
  localparam int N = 2 + (4'bx100 ==? 4'b1?00);
  initial begin $display("G7 N=%0d", N); end
endmodule
