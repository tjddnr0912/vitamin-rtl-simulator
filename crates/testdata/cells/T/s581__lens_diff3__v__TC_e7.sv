module top;
  localparam logic [64:0] P65 = 65'hC;
  localparam int R = (P65 ==? 4'b1?00) ? 10 : 20;
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
