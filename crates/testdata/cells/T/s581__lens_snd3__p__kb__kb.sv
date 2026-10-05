module t;
`ifdef K1
  localparam L = (4'd12 ==? 8'b1000_1?00);
`elsif K2
  localparam L = ((4'd15 + 4'd1) ==? 5'b0?000);
`elsif K3
  localparam L = ((4'd15 + 4'd1) ==? 5'b1?000);
`endif
  initial begin $display("L=%0d", L); $finish; end
endmodule
