module top;
  localparam logic [64:0] P65 = 65'hC;
  localparam logic [7:0] V = 8'b1011_0110;
  localparam logic [3:0] R = V[(P65 ==? 4'b1?00) * 4 +: 4];
  initial $display("R=%b", R);
  initial #100 $finish;
endmodule
