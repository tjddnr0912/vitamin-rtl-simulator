module t;
  localparam signed [3:0] P = -6;
  localparam int A = (P ==? 4'b1x1x);
  localparam int A2 = (P ==? 4'b0x1x);
  localparam logic [39:0] Q = 40'hFF_0000_0000;
  localparam int M = (Q ==? 'hx);
  localparam int M2 = (Q ==? 'h0x);
  initial $display("A=%0d A2=%0d M=%0d M2=%0d", A, A2, M, M2);
endmodule
