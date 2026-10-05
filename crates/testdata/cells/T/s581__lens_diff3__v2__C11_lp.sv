module top;
  localparam int N2 = 2;
  localparam R = ({N2{4'b1100}} == 8'b1100_1100);
  initial $display("R=%0d", R);
  initial #100 $finish;
endmodule
