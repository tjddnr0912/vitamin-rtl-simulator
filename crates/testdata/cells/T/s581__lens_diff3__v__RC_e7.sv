module top;
  localparam logic [64:0] P65 = 65'hC;
  localparam logic [7:0] R = {(P65 ==? 4'b1?00) + 1 {4'b1010}};
  initial $display("R=%b", R);
  initial #100 $finish;
endmodule
