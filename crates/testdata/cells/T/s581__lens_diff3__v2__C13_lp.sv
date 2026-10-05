module top;
  localparam int N2 = 2;
  localparam R = ({N2{40'hC}} ==? 4'b1?00);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
