module t;
  typedef enum logic [7:0] { EA = 8'b1x01_0000, EB = 8'd1 } e_t;
  localparam logic [7:0] R = EA;
  localparam e_t S = EA;
  initial begin #1 $display("A r=%b s=%b", R, S); end
  initial #100 $finish;
endmodule
