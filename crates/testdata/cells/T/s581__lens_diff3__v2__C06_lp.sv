module top;
  localparam int N2 = 2;
  localparam R = ({N2{2'b11}} ==? 4'b1?11);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
