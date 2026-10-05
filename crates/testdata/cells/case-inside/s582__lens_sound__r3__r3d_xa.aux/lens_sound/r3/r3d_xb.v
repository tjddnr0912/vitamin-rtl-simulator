module child;
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase $display("r3d child m=%0d", m); end
endmodule
