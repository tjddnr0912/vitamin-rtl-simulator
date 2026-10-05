module top;
  parameter P = 4'b1111;
  localparam L = ((P + 4'd1) ==? 4'b000?);
  initial $display("I1 L=%0d", L);
endmodule
